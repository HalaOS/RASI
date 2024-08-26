use std::{
    io::{self, Result},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::Poll,
};

use futures::{future::BoxFuture, lock::Mutex, AsyncRead, AsyncWrite, FutureExt};
use futures_map::KeyWaitMap;
use rasi::task::spawn_ok;

use crate::{Error, Reason, Session};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum ConnEvent {
    Send,
    StreamRecv(u32),
    StreamSend(u32),
    Accept,
}

#[derive(Default)]
struct YamuxStreamDropTable {
    count: AtomicUsize,
    stream_ids: std::sync::Mutex<Vec<u32>>,
}

impl YamuxStreamDropTable {
    /// Push new stream id into this drop table.
    fn push(&self, stream_id: u32) {
        self.stream_ids.lock().unwrap().push(stream_id);
        self.count.fetch_add(1, Ordering::Release);
    }

    fn drain(&self) -> Option<Vec<u32>> {
        if self.count.load(Ordering::Acquire) == 0 {
            return None;
        }

        let drain = self
            .stream_ids
            .lock()
            .unwrap()
            .drain(..)
            .collect::<Vec<_>>();

        self.count.fetch_sub(drain.len(), Ordering::Release);

        Some(drain)
    }
}

/// Yamux connection type with asynchronous api.
#[derive(Clone)]
pub struct YamuxConnState {
    is_closed: Arc<AtomicBool>,
    session: Arc<Mutex<Session>>,
    event_map: Arc<KeyWaitMap<ConnEvent, ()>>,
    is_server: bool,
    stream_drop_table: Arc<YamuxStreamDropTable>,
}

impl YamuxConnState {
    fn notify_stream_events<S>(&self, session: &S)
    where
        S: Deref<Target = Session>,
    {
        let mut events = vec![];

        for id in session.readable() {
            events.push((ConnEvent::StreamRecv(id), ()));
        }

        for id in session.writable() {
            events.push((ConnEvent::StreamSend(id), ()));
        }

        if session.acceptable() {
            events.push((ConnEvent::Accept, ()));
        }

        self.event_map.batch_insert(events);
    }

    async fn handle_stream_drop(&self, state: &mut Session) -> bool {
        if let Some(drain) = self.stream_drop_table.drain() {
            let drop_streams = drain.len();
            for stream_id in drain {
                if let Err(err) = state.stream_send(stream_id, b"", true) {
                    log::error!("drop stream failed, stream_id={}, error={}", stream_id, err);
                }

                if !state.stream_finished(stream_id) {
                    if let Err(err) = state.stream_reset(stream_id) {
                        log::error!("drop stream failed, stream_id={}, error={}", stream_id, err);
                    }
                }
            }

            return drop_streams != 0;
        }

        return false;
    }

    /// Write new frame to be sent to peer into provided slice.
    ///
    pub async fn send(&self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut session = self.session.lock().await;

            if self.is_closed() {
                if !session.is_closed() {
                    session.close(Reason::Normal)?;
                    self.event_map.cancel_all();
                }
            }

            match session.send(buf) {
                Ok(send_size) => {
                    self.notify_stream_events(&session);

                    return Ok(send_size);
                }
                Err(Error::Done) => {
                    if self.handle_stream_drop(&mut session).await {
                        drop(session);
                        continue;
                    }

                    log::trace!("send data. waiting");

                    self.event_map.wait(&ConnEvent::Send, session).await;

                    continue;
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    /// Write new data received from peer.
    pub async fn recv(&self, buf: &[u8]) -> crate::errors::Result<usize> {
        let mut session = self.session.lock().await;
        match session.recv(buf) {
            Ok(send_size) => {
                self.notify_stream_events(&session);
                return Ok(send_size);
            }

            Err(err) => return Err(err.into()),
        }
    }

    /// Close `YamuxConn` with provided [`reason`](Reason)
    pub fn close(&self, _reason: Reason) -> io::Result<()> {
        self.is_closed.store(true, Ordering::SeqCst);

        self.event_map.insert(ConnEvent::Send, ());

        Ok(())
    }

    /// Get the closed flag.
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::SeqCst)
    }

    async fn stream_send_owned(self, stream_id: u32, buf: Vec<u8>, fin: bool) -> io::Result<usize> {
        self.stream_send(stream_id, &buf, fin).await
    }

    pub async fn stream_send(&self, stream_id: u32, buf: &[u8], fin: bool) -> io::Result<usize> {
        loop {
            let mut session = self.session.lock().await;
            match session.stream_send(stream_id, buf, fin) {
                Ok(send_size) => {
                    self.event_map.insert(ConnEvent::Send, ());
                    return Ok(send_size);
                }
                Err(Error::Done) => {
                    self.event_map
                        .wait(&ConnEvent::StreamSend(stream_id), session)
                        .await;

                    continue;
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    async fn stream_recv_owned(self, stream_id: u32, len: usize) -> io::Result<(Vec<u8>, bool)> {
        let mut buf = vec![0; len];

        let (read_size, fin) = self.stream_recv(stream_id, &mut buf).await?;

        buf.resize(read_size, 0);

        Ok((buf, fin))
    }

    pub async fn stream_recv(&self, stream_id: u32, buf: &mut [u8]) -> io::Result<(usize, bool)> {
        loop {
            log::trace!("stream, id={}, recv", stream_id);
            let mut session = self.session.lock().await;
            match session.stream_recv(stream_id, buf) {
                Ok((send_size, fin)) => {
                    log::trace!(
                        "stream, id={}, recv_size={}, fin={}",
                        stream_id,
                        send_size,
                        fin
                    );
                    self.event_map.insert(ConnEvent::Send, ());
                    return Ok((send_size, fin));
                }
                Err(Error::Done) => {
                    log::trace!("stream, id={}, recv pending.", stream_id,);

                    self.event_map
                        .wait(&ConnEvent::StreamRecv(stream_id), session)
                        .await;

                    continue;
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    pub async fn stream_accept(&self) -> io::Result<YamuxStream> {
        loop {
            let mut session = self.session.lock().await;
            if let Some(stream_id) = session.accept()? {
                log::trace!("stream accept, id={}", stream_id);
                return Ok(YamuxStream::new(stream_id, self.clone()));
            }

            log::trace!(target:"YamuxConn","Accept waiting..");

            self.event_map.wait(&ConnEvent::Accept, session).await;

            log::trace!(target:"YamuxConn","Accept wakeup");
        }
    }

    /// Open new outbound stream.
    pub async fn stream_open(&self) -> io::Result<YamuxStream> {
        let mut session = self.session.lock().await;

        let stream_id = session.open()?;

        self.event_map.insert(ConnEvent::Send, ());

        Ok(YamuxStream::new(stream_id, self.clone()))
    }

    /// Returns true if all the data has been read from the specified stream.
    ///
    /// This instructs the application that all the data received from the peer on the stream has been read, and there won’t be anymore in the future.
    ///
    /// Basically this returns true when the peer either set the fin flag for the stream, or sent *_FRAME with RST flag.
    pub async fn stream_finished(&self, stream_id: u32) -> bool {
        let session = self.session.lock().await;

        session.stream_finished(stream_id)
    }

    /// Elegantly close stream.
    pub fn stream_close(&self, stream_id: u32) -> io::Result<()> {
        self.stream_drop_table.push(stream_id);
        self.event_map.insert(ConnEvent::Send, ());

        Ok(())
    }
}

impl YamuxConnState {
    /// Create yamux `Conn` instance with provided parameters.
    pub fn new(window_size: u32, is_server: bool) -> Self {
        let session = Session::new(window_size, is_server);

        let conn = YamuxConnState {
            is_closed: Default::default(),
            session: Arc::new(Mutex::new(session)),
            event_map: Arc::new(KeyWaitMap::new()),
            is_server,
            stream_drop_table: Default::default(),
        };

        conn
    }
}

/// Yamux connection type with asynchronous api.
pub struct YamuxConn(YamuxConnState);

impl YamuxConn {
    /// Create yamux `Conn` instance with provided parameters.
    pub fn new(window_size: u32, is_server: bool) -> Self {
        Self(YamuxConnState::new(window_size, is_server))
    }

    /// Get inner [`YamuxConnState`] instance.
    pub fn to_state(&self) -> YamuxConnState {
        self.0.clone()
    }
}

impl Deref for YamuxConn {
    type Target = YamuxConnState;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for YamuxConn {
    fn drop(&mut self) {
        _ = self.0.close(Reason::Normal);
    }
}

#[cfg(feature = "rasi")]
impl YamuxConn {
    /// Create a new yamux `Conn` instance with reliable stream underneath.
    ///
    /// This function will start two event loops:
    ///
    /// - message send loop, read yamux frame from session and send to peer.
    /// - message recv loop, recv yamux frame from peer and write to session.
    pub fn new_with<R, W>(window_size: u32, is_server: bool, reader: R, writer: W) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let conn = Self::new(window_size, is_server);

        // spawn the recv loop
        spawn_ok(Self::recv_loop(conn.clone(), reader));
        // spawn the send loop
        spawn_ok(Self::send_loop(conn.clone(), writer));

        conn
    }

    async fn recv_loop<R>(mut conn: YamuxConnState, mut reader: R)
    where
        R: AsyncRead + Unpin + Send,
    {
        match Self::recv_loop_inner(&mut conn, &mut reader).await {
            Ok(_) => {
                log::trace!("stop recv loop");
            }
            Err(err) => {
                log::error!("stop recv loop with error: {}", err);
            }
        }

        // Close session.
        _ = conn.close(Reason::Normal);
    }

    async fn recv_loop_inner<R>(conn: &mut YamuxConnState, reader: &mut R) -> io::Result<()>
    where
        R: AsyncRead + Unpin + Send,
    {
        use futures::io::AsyncReadExt;

        let mut buf = vec![0; 1024 * 4 + 12];

        loop {
            log::trace!("recv frame header, is_server={}", conn.is_server);

            reader.read_exact(&mut buf[0..12]).await?;

            log::trace!("recv frame header, is_server={}, Ok", conn.is_server);

            match conn.recv(&buf[..12]).await {
                Ok(_) => {
                    continue;
                }
                Err(Error::BufferTooShort(len)) => {
                    if len > buf.len() as u32 {
                        return Err(Error::Overflow.into());
                    }

                    log::trace!("recv data frame body, len={}", len - 12);

                    reader.read_exact(&mut buf[12..len as usize]).await?;

                    log::trace!("recv data frame body, len={}, Ok", len - 12);

                    conn.recv(&buf[..len as usize]).await?;
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    async fn send_loop<W>(mut conn: YamuxConnState, mut writer: W)
    where
        W: AsyncWrite + Unpin + Send,
    {
        match Self::send_loop_inner(&mut conn, &mut writer).await {
            Ok(_) => {
                log::trace!("Yamux conn stop send loop");
            }
            Err(err) => {
                log::error!("Yamux conn stop send loop, {}", err);
            }
        }

        use futures::io::AsyncWriteExt;

        _ = writer.close().await;
    }
    async fn send_loop_inner<W>(conn: &mut YamuxConnState, writer: &mut W) -> io::Result<()>
    where
        W: AsyncWrite + Unpin + Send,
    {
        use futures::io::AsyncWriteExt;

        let mut buf = vec![0; 1024 + 12];

        loop {
            let send_size: usize = conn.send(&mut buf).await?;

            writer.write_all(&buf[..send_size]).await?;

            buf[..12].fill(0x0);

            log::trace!(
                "yamux send loop, transfer data to peer, is_server={}, len={}",
                conn.is_server,
                send_size
            );
        }
    }
}

/// Stream object with [`Drop`] trait.
struct RawStream(u32, YamuxConnState);

impl Drop for RawStream {
    fn drop(&mut self) {
        let stream_id = self.0;
        let conn = self.1.clone();

        _ = conn.stream_close(stream_id);
    }
}

enum StreamPoll {
    PollClose(BoxFuture<'static, Result<usize>>),
    PollSend(BoxFuture<'static, Result<usize>>),
    PollRead(BoxFuture<'static, Result<(Vec<u8>, bool)>>),
}

/// Yamux stream type with asynchronous api.
pub struct YamuxStream {
    raw: Arc<RawStream>,
    poll: Option<StreamPoll>,
}

unsafe impl Send for YamuxStream {}
unsafe impl Sync for YamuxStream {}

impl YamuxStream {
    fn new(stream_id: u32, conn: YamuxConnState) -> Self {
        Self {
            raw: Arc::new(RawStream(stream_id, conn)),
            poll: None,
        }
    }

    /// Returns stream id.
    pub fn stream_id(&self) -> u32 {
        self.raw.0
    }
}

impl AsyncWrite for YamuxStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize>> {
        let mut fut = if let Some(StreamPoll::PollSend(fut)) = self.poll.take() {
            fut
        } else {
            Box::pin(
                self.raw
                    .1
                    .clone()
                    .stream_send_owned(self.raw.0, buf.to_owned(), false),
            )
        };

        match fut.poll_unpin(cx) {
            Poll::Pending => {
                self.poll = Some(StreamPoll::PollSend(fut));

                Poll::Pending
            }

            r => r,
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        let mut fut = if let Some(StreamPoll::PollClose(fut)) = self.poll.take() {
            fut
        } else {
            Box::pin(
                self.raw
                    .1
                    .clone()
                    .stream_send_owned(self.stream_id(), vec![], true),
            )
        };

        match fut.poll_unpin(cx) {
            Poll::Pending => {
                self.poll = Some(StreamPoll::PollClose(fut));

                Poll::Pending
            }

            Poll::Ready(r) => Poll::Ready(r.map(|_| ())),
        }
    }
}

impl AsyncRead for YamuxStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<Result<usize>> {
        let mut fut = if let Some(StreamPoll::PollRead(fut)) = self.poll.take() {
            fut
        } else {
            Box::pin(
                self.raw
                    .1
                    .clone()
                    .stream_recv_owned(self.stream_id(), buf.len()),
            )
        };

        match fut.poll_unpin(cx) {
            Poll::Ready(Ok((packet, _))) => {
                buf[..packet.len()].copy_from_slice(&packet);

                Poll::Ready(Ok(packet.len()))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => {
                self.poll = Some(StreamPoll::PollRead(fut));
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Once, time::Duration};

    use futures::{AsyncReadExt, AsyncWriteExt};
    use rasi::{
        net::{TcpListener, TcpStream},
        timer::sleep,
    };
    use rasi_mio::{net::register_mio_network, timer::register_mio_timer};

    use crate::INIT_WINDOW_SIZE;

    use super::*;

    fn init() {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            register_mio_network();
            register_mio_timer();

            pretty_env_logger::init_timed();
        });
    }

    #[futures_test::test]
    async fn test_conn() {
        init();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        let local_addr = listener.local_addr().unwrap();

        spawn_ok(async move {
            let (stream, _) = listener.accept().await.unwrap();

            let (read, write) = stream.split();

            let conn = YamuxConn::new_with(INIT_WINDOW_SIZE, true, read, write);

            let stream = conn.stream_accept().await.unwrap();

            assert_eq!(stream.stream_id(), 1);

            let mut stream = conn.stream_open().await.unwrap();

            stream.write_all(b"hello world").await.unwrap();

            let mut buf = vec![0; 12];

            _ = stream.read(&mut buf).await;
        });

        let state = {
            let (read, write) = TcpStream::connect(local_addr).await.unwrap().split();

            let conn = YamuxConn::new_with(INIT_WINDOW_SIZE, false, read, write);

            let stream = conn.stream_open().await.unwrap();

            assert_eq!(stream.stream_id(), 1);

            let mut stream = conn.stream_accept().await.unwrap();

            assert_eq!(stream.stream_id(), 2);

            let mut buf = vec![0; 100];

            let read_size = stream.read(&mut buf).await.unwrap();

            assert_eq!(&buf[..read_size], b"hello world");

            conn.to_state()
        };

        sleep(Duration::from_secs(1)).await;

        assert_eq!(Arc::strong_count(&state.session), 1);
    }
}
