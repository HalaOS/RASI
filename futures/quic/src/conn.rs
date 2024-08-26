use std::{
    collections::{HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
    io::{Error, ErrorKind, Result},
    net::SocketAddr,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::Poll,
    time::{Duration, Instant},
};

use futures::{future::BoxFuture, lock::Mutex, AsyncRead, AsyncWrite, FutureExt, Stream};
use futures_map::KeyWaitMap;
use quiche::{ConnectionId, RecvInfo, SendInfo};

use crate::errors::map_quic_error;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum QuicConnStateEvent {
    /// This event notify listener that this state machine is now readable.
    Send(ConnectionId<'static>),

    /// This event notify listener that one stream of this state machine is now readable.
    StreamReadable(ConnectionId<'static>, u64),

    /// This event notify listener that one stream of this state machine is now writable.
    StreamWritable(ConnectionId<'static>, u64),

    /// This event notify listener that one incoming stream is valid.
    StreamAccept(ConnectionId<'static>),
    /// This event notify that peer_streams_left_bidi > 0
    OutboundStream(ConnectionId<'static>),
}

struct QuicRawConnState {
    /// Quiche connection statement.
    conn: quiche::Connection,
    /// Next stream outbound id.
    next_outbound_stream_id: u64,
    /// Known unclosed inbound streams
    inbound_stream_ids: HashSet<u64>,
    /// Known outbound streams that had never sent any data.
    outbound_stream_ids: HashSet<u64>,
    /// When first see a inbound stream, push it into this queue.
    incoming_streams: VecDeque<u64>,
    /// The time interval for sending ack_eliciting packets.
    /// To disable the behaviour of sending ack_eliciting packets, set this field to [`None`].
    ack_eliciting_interval: Option<Duration>,
    /// The time of latest sending ack_eliciting packets.
    latest_send_ack_eliciting_at: Instant,
}

impl Debug for QuicRawConnState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "QuicConn scid={:?}, dcid={:?}, is_server={}",
            self.conn.source_id(),
            self.conn.destination_id(),
            self.conn.is_server()
        )
    }
}

impl QuicRawConnState {
    fn new(
        conn: quiche::Connection,
        init_stream_outbound_id: u64,
        ack_eliciting_interval: Option<Duration>,
    ) -> Self {
        QuicRawConnState {
            conn,
            next_outbound_stream_id: init_stream_outbound_id,
            inbound_stream_ids: Default::default(),
            outbound_stream_ids: Default::default(),
            incoming_streams: Default::default(),
            ack_eliciting_interval,
            latest_send_ack_eliciting_at: Instant::now(),
        }
    }
}

#[derive(Default)]
struct QuicStreamDropTable {
    count: AtomicUsize,
    stream_ids: std::sync::Mutex<Vec<u64>>,
}

impl QuicStreamDropTable {
    /// Push new stream id into this drop table.
    fn push(&self, stream_id: u64) {
        self.stream_ids.lock().unwrap().push(stream_id);
        self.count.fetch_add(1, Ordering::Release);
    }

    fn drain(&self) -> Option<Vec<u64>> {
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

/// A Quic connection between a local and a remote socket.
#[derive(Clone)]
pub struct QuicConnState {
    max_send_udp_payload_size: usize,
    pub(crate) id: ConnectionId<'static>,
    is_closed: Arc<AtomicBool>,
    state: Arc<Mutex<QuicRawConnState>>,
    event_map: Arc<KeyWaitMap<QuicConnStateEvent, ()>>,
    stream_drop_table: Arc<QuicStreamDropTable>,
}

impl Debug for QuicConnState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "quic conn, id={:?}", self.id)
    }
}

impl Hash for QuicConnState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl PartialEq for QuicConnState {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Eq for QuicConnState {}

impl QuicConnState {
    async fn after_recv(&self, state: &mut QuicRawConnState) {
        // reset the `send_ack_eliciting_instant`
        state.latest_send_ack_eliciting_at = Instant::now();

        let mut raised_events = vec![
            // as [`send`](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.send)
            // function description, that is, any time recv() is called, we should call send() again.
            (
                QuicConnStateEvent::Send(state.conn.source_id().into_owned()),
                (),
            ),
        ];

        self.collect_stream_events(state, &mut raised_events);

        self.event_map.batch_insert(raised_events);

        self.handle_stream_drop(state).await;
    }
    /// inner call this method after success send one packet.
    async fn after_send(&self, state: &mut QuicRawConnState) {
        let mut raised_events = vec![];

        self.collect_stream_events(state, &mut raised_events);

        self.event_map.batch_insert(raised_events);

        self.handle_stream_drop(state).await;
    }

    async fn send_ack_eliciting(&self, state: &mut QuicRawConnState) -> Result<bool> {
        if let Some(ack_eliciting_interval) = state.ack_eliciting_interval {
            if state.latest_send_ack_eliciting_at.elapsed() >= ack_eliciting_interval {
                state.conn.send_ack_eliciting().map_err(map_quic_error)?;

                // send `ack_eliciting` immediately.
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Mark the provided `stream_id` to be drop.
    fn stream_close(&self, stream_id: u64) {
        self.stream_drop_table.push(stream_id);
        self.event_map
            .insert(QuicConnStateEvent::Send(self.id.clone()), ());
    }

    async fn handle_stream_drop(&self, state: &mut QuicRawConnState) -> bool {
        if let Some(drain) = self.stream_drop_table.drain() {
            let drop_streams = drain.len();
            for stream_id in drain {
                if let Err(err) = state.conn.stream_send(stream_id, b"", true) {
                    log::error!(
                        "{:?}, drop stream failed, stream_id={}, error={}",
                        state,
                        stream_id,
                        err
                    );
                }

                if !state.conn.stream_finished(stream_id) {
                    if let Err(err) =
                        state
                            .conn
                            .stream_shutdown(stream_id, quiche::Shutdown::Read, 0)
                    {
                        log::error!(
                            "{:?}, drop stream failed, stream_id={}, error={}",
                            state,
                            stream_id,
                            err
                        );
                    }
                }
            }

            return drop_streams != 0;
        }

        return false;
    }

    fn collect_stream_events(
        &self,
        state: &mut QuicRawConnState,
        raised_events: &mut Vec<(QuicConnStateEvent, ())>,
    ) {
        for stream_id in state.conn.readable() {
            // check if the stream is a new inbound stream.
            // If true push stream into the acceptance queue instead of triggering a readable event
            if stream_id % 2 != state.next_outbound_stream_id % 2
                && !state.inbound_stream_ids.contains(&stream_id)
            {
                state.inbound_stream_ids.insert(stream_id);
                state.incoming_streams.push_back(stream_id);

                log::trace!("{:?}, accept a new inbound stream id={}", state, stream_id);

                raised_events.push((
                    QuicConnStateEvent::StreamAccept(state.conn.source_id().into_owned()),
                    (),
                ));

                continue;
            }

            raised_events.push((
                QuicConnStateEvent::StreamReadable(state.conn.source_id().into_owned(), stream_id),
                (),
            ));
        }

        for stream_id in state.conn.writable() {
            raised_events.push((
                QuicConnStateEvent::StreamWritable(state.conn.source_id().into_owned(), stream_id),
                (),
            ));
        }
    }

    fn peer_streams_left_bidi_priv(&self, state: &QuicRawConnState) -> u64 {
        let peer_streams_left_bidi = state.conn.peer_streams_left_bidi();
        let outgoing_cached = state.outbound_stream_ids.len() as u64;
        let initial_max_streams_bidi = state
            .conn
            .peer_transport_params()
            .unwrap()
            .initial_max_streams_bidi;

        if peer_streams_left_bidi > outgoing_cached {
            if initial_max_streams_bidi == peer_streams_left_bidi {
                peer_streams_left_bidi - outgoing_cached - 1
            } else {
                peer_streams_left_bidi - outgoing_cached
            }
        } else {
            0
        }
    }

    pub(crate) async fn send_owned(self) -> Result<(Vec<u8>, SendInfo)> {
        let mut buf = vec![0; self.max_send_udp_payload_size];

        log::trace!("send_owned, scid={:?}", self.id);

        let (send_size, send_info) = self.send(&mut buf).await?;

        log::trace!("send_owned, scid={:?}, len={:?}", self.id, send_size);
        // Safety: send method ensure send_size <= buf.len()
        buf.resize(send_size, 0);

        Ok((buf, send_info))
    }

    pub(crate) async fn on_timeout(&self) {
        self.state.lock().await.conn.on_timeout();
    }
}

impl QuicConnState {
    /// Create a new `QuicConn` instance.
    pub fn new(
        inner: quiche::Connection,
        init_stream_outbound_id: u64,
        ack_eliciting_interval: Option<Duration>,
    ) -> Self {
        Self {
            is_closed: Default::default(),
            max_send_udp_payload_size: inner.max_send_udp_payload_size(),
            id: inner.source_id().into_owned(),
            state: Arc::new(Mutex::new(QuicRawConnState::new(
                inner,
                init_stream_outbound_id,
                ack_eliciting_interval,
            ))),
            event_map: Arc::new(KeyWaitMap::new()),
            stream_drop_table: Default::default(),
        }
    }

    /// Return true if this connection handshake is complete.
    pub async fn is_established(&self) -> bool {
        self.state.lock().await.conn.is_established()
    }

    /// Returns when the next timeout event will occur.
    ///
    /// Once the timeout Instant has been reached, the `on_timeout()` method
    /// should be called. A timeout of `None` means that the timer should be
    /// disarmed.
    ///
    pub async fn timeout_instant(&self) -> Option<Instant> {
        self.state.lock().await.conn.timeout_instant()
    }

    /// Read sending data from the `QuicConn`.
    ///
    /// On success, returns the total number of bytes copied and the [`SendInfo`]
    pub async fn send(&self, buf: &mut [u8]) -> Result<(usize, SendInfo)> {
        let mut on_timout = false;

        loop {
            let mut state = self.state.lock().await;

            if self.is_closed.load(Ordering::SeqCst) {
                if !state.conn.is_closed() {
                    _ = state.conn.close(false, 0, b"");
                }
            }

            // generate timeout packet.
            if on_timout {
                log::trace!("{:?}, send data, timeout", *state);
                on_timout = false;
                state.conn.on_timeout();
            }

            match state.conn.send(buf) {
                Ok((send_size, send_info)) => {
                    log::trace!(
                        "{:?}, send data, len={}, elapsed={:?}",
                        *state,
                        send_size,
                        send_info.at.elapsed()
                    );

                    self.after_send(&mut state).await;

                    return Ok((send_size, send_info));
                }
                Err(quiche::Error::Done) => {
                    if state.conn.is_closed() {
                        self.event_map.cancel_all();
                        return Err(Error::new(
                            ErrorKind::BrokenPipe,
                            format!("connection is draining: {:?}", *state),
                        ));
                    }
                    // No more data to send and conn is not established,
                    // indicate that the connection idle timeout expired.
                    if !state.conn.is_established() && state.conn.is_timed_out() {
                        self.event_map.cancel_all();

                        log::trace!(
                            "{:?} idle timeout expired, timeout={:?}",
                            *state,
                            state.conn.timeout_instant()
                        );

                        return Err(Error::new(
                            ErrorKind::TimedOut,
                            format!("connect timeout: {:?}", *state),
                        ));
                    }

                    if self.handle_stream_drop(&mut state).await {
                        drop(state);
                        continue;
                    }

                    if self.send_ack_eliciting(&mut state).await? {
                        drop(state);
                        continue;
                    }

                    let event =
                        QuicConnStateEvent::Send(state.conn.source_id().into_owned().clone());

                    use rasi::timer::TimeoutExt;

                    if let Some(timeout_at) = state.conn.timeout_instant() {
                        if self
                            .event_map
                            .wait(&event, state)
                            .timeout_at(timeout_at)
                            .await
                            .is_none()
                        {
                            on_timout = true;
                        }
                    } else {
                        self.event_map.wait(&event, state).await;
                    }

                    log::trace!("QuicConn scid={:?}, send wakeup", self.id);

                    continue;
                }
                Err(err) => return Err(map_quic_error(err)),
            }
        }
    }

    /// Processes QUIC packets received from the peer.
    ///
    /// On success the number of bytes processed from the input buffer is returned.
    pub async fn recv(&self, buf: &mut [u8], info: RecvInfo) -> Result<usize> {
        let mut state = self.state.lock().await;

        match state.conn.recv(buf, info) {
            Ok(read_size) => {
                log::trace!("{:?}, recv packet, len={}", state.deref(), read_size);

                self.after_recv(&mut state).await;

                Ok(read_size)
            }
            Err(err) => {
                // On error the connection will be closed by calling close() with the appropriate error code.
                // see quiche [document](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.recv)
                // for more information.

                // So we must cancel all pending listener of event_map immediately;
                self.event_map.cancel_all();

                Err(map_quic_error(err))
            }
        }
    }

    /// Accept a new inbound stream.
    pub async fn accept(&self) -> Result<QuicStream> {
        loop {
            if self.is_closed.load(Ordering::SeqCst) {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "Underly quiche connection is closed.",
                ));
            }

            let mut state = self.state.lock().await;

            self.handle_stream_drop(&mut state).await;

            if let Some(stream_id) = state.incoming_streams.pop_front() {
                return Ok(QuicStream::new(stream_id, self.clone()));
            }

            let event = QuicConnStateEvent::StreamAccept(state.conn.source_id().into_owned());

            log::trace!("accept new incoming stream -- waiting");

            self.event_map.wait(&event, state).await;

            log::trace!("accept new incoming stream -- wakeup");
        }
    }

    /// Open a new outbound stream over this connection.
    ///
    /// # nonblocking
    ///
    /// When nonblocking parameter is true and the `peer_streams_left_bidi`] is zero,
    /// this function will cause an [`ErrorKind::WouldBlock`] error.
    pub async fn open(&self, nonblocking: bool) -> Result<QuicStream> {
        loop {
            let mut state = self.state.lock().await;

            self.handle_stream_drop(&mut state).await;

            let peer_streams_left_bidi = self.peer_streams_left_bidi_priv(&state);

            if peer_streams_left_bidi == 0 {
                if nonblocking {
                    return Err(Error::new(
                        ErrorKind::WouldBlock,
                        quiche::Error::StreamLimit,
                    ));
                }

                let event = QuicConnStateEvent::OutboundStream(state.conn.source_id().into_owned());

                self.event_map.wait(&event, state).await;

                continue;
            }

            let stream_id = state.next_outbound_stream_id;

            state.next_outbound_stream_id += 4;

            // removed after first call to stream_send.
            state.outbound_stream_ids.insert(stream_id);

            return Ok(QuicStream::new(stream_id, self.clone()));
        }
    }

    /// Returns the peer's cert in der format if valid.
    pub async fn peer_cert(&self) -> Option<Vec<u8>> {
        let state = self.state.lock().await;

        state.conn.peer_cert().map(|v| v.to_vec())
    }

    /// Get the peer address on path.
    pub async fn peer_addr(&self, laddr: SocketAddr) -> Option<SocketAddr> {
        let state = self.state.lock().await;

        state.conn.paths_iter(laddr).next()
    }

    pub async fn path(&self) -> Option<(SocketAddr, SocketAddr)> {
        let state = self.state.lock().await;

        let result = state
            .conn
            .path_stats()
            .next()
            .map(|stats| (stats.local_addr, stats.peer_addr));

        result
    }

    /// Get this source id of this connection.
    pub fn scid(&self) -> &ConnectionId<'_> {
        &self.id
    }

    /// Close this connection.
    pub fn close(&self) -> Result<()> {
        self.is_closed.store(true, Ordering::SeqCst);

        self.event_map
            .insert(QuicConnStateEvent::Send(self.id.clone()), ());
        self.event_map
            .insert(QuicConnStateEvent::StreamAccept(self.id.clone()), ());

        Ok(())
    }
}

/// The quic connection instance type with auto [`Drop`] support.
pub struct QuicConn(QuicConnState);

impl Drop for QuicConn {
    fn drop(&mut self) {
        _ = self.0.close();
    }
}

impl From<QuicConnState> for QuicConn {
    fn from(value: QuicConnState) -> Self {
        Self(value)
    }
}

impl Deref for QuicConn {
    type Target = QuicConnState;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl QuicConn {
    /// Returns a stream of incoming connections.
    ///
    /// Iterating over this stream is equivalent to calling accept in a loop.
    /// The stream of connections is infinite, i.e awaiting the next connection
    /// will never result in None.
    pub fn into_incoming(self) -> impl Stream<Item = Result<QuicStream>> + Unpin {
        Box::pin(futures::stream::unfold(self, |conn| async move {
            let res = conn.accept().await;
            Some((res, conn))
        }))
    }
}

struct RawQuicStream {
    stream_id: u64,
    conn: QuicConnState,
}

impl Drop for RawQuicStream {
    fn drop(&mut self) {
        self.conn.stream_close(self.stream_id);
    }
}

impl RawQuicStream {
    /// Writes data to a stream.
    ///
    /// On success the number of bytes written is returned.
    pub async fn send<Buf: AsRef<[u8]>>(&self, buf: Buf, fin: bool) -> Result<usize> {
        let buf = buf.as_ref();

        loop {
            let mut state = self.conn.state.lock().await;

            if state.conn.is_closed() || state.conn.is_draining() {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    format!("connection is closed/draining: {:?}", *state),
                ));
            }

            let result = state.conn.stream_send(self.stream_id, buf, fin);

            // After calling stream_send,
            // The [`peer_streams_left_bidi`](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.peer_streams_left_bidi)
            // will be able to return the correct value.
            //
            // So the outbound stream id can be removed from `outbound_stream_ids`, safely.
            if self.stream_id % 2 == state.next_outbound_stream_id % 2 {
                // notify can open next stream.
                if state.outbound_stream_ids.remove(&self.stream_id) {
                    self.conn.event_map.insert(
                        QuicConnStateEvent::OutboundStream(state.conn.source_id().into_owned()),
                        (),
                    );
                }
            }

            match result {
                Ok(send_size) => {
                    // According to the function A [`send`](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.send)
                    // document description, we should call the send function immediately.
                    self.conn.event_map.insert(
                        QuicConnStateEvent::Send(state.conn.source_id().into_owned()),
                        (),
                    );

                    return Ok(send_size);
                }
                Err(quiche::Error::Done) => {
                    // if no data was written(e.g. because the stream has no capacity),
                    // call `send()` function immediately
                    self.conn.event_map.insert(
                        QuicConnStateEvent::Send(state.conn.source_id().into_owned()),
                        (),
                    );

                    let event = QuicConnStateEvent::StreamWritable(
                        state.conn.source_id().into_owned(),
                        self.stream_id,
                    );

                    self.conn.event_map.wait(&event, state).await;

                    continue;
                }
                Err(err) => return Err(map_quic_error(err)),
            }
        }
    }

    async fn recv<Buf: AsMut<[u8]>>(&self, mut buf: Buf) -> Result<(usize, bool)> {
        let buf = buf.as_mut();

        loop {
            let mut state = self.conn.state.lock().await;

            if state.conn.is_closed() || state.conn.is_draining() {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    format!("connection is closed/draining: {:?}", *state),
                ));
            }

            match state.conn.stream_recv(self.stream_id, buf) {
                Ok((read_size, fin)) => {
                    self.conn.event_map.insert(
                        QuicConnStateEvent::Send(state.conn.source_id().into_owned()),
                        (),
                    );

                    return Ok((read_size, fin));
                }
                Err(quiche::Error::Done) => {
                    let event = QuicConnStateEvent::StreamReadable(
                        state.conn.source_id().into_owned(),
                        self.stream_id,
                    );

                    self.conn.event_map.wait(&event, state).await;
                    log::trace!("stream wakeup: {:?}", event);
                    continue;
                }
                Err(quiche::Error::InvalidStreamState(_)) => {
                    // the stream is not created yet.
                    if state.outbound_stream_ids.contains(&self.stream_id) {
                        let event = QuicConnStateEvent::StreamReadable(
                            state.conn.source_id().into_owned(),
                            self.stream_id,
                        );

                        self.conn.event_map.wait(&event, state).await;
                    } else {
                        return Err(map_quic_error(quiche::Error::InvalidStreamState(
                            self.stream_id,
                        )));
                    }
                }
                Err(err) => {
                    log::trace!("{:?}, recv with error, {}", *state, err);
                    return Err(map_quic_error(err));
                }
            }
        }
    }

    async fn send_owned(self: Arc<Self>, buf: Vec<u8>, fin: bool) -> Result<usize> {
        self.send(buf, fin).await
    }

    async fn recv_owned(self: Arc<Self>, len: usize) -> Result<(Vec<u8>, bool)> {
        let mut buf = vec![0; len];

        let (recv_size, fin) = self.recv(&mut buf).await?;

        buf.resize(recv_size, 0);

        Ok((buf, fin))
    }
}

/// The AsyncRead / AsyncWrite poll state matchine.
enum QuicStreamPoll {
    PollWrite(BoxFuture<'static, Result<usize>>),
    PollRead(BoxFuture<'static, Result<(Vec<u8>, bool)>>),
    PollClose(BoxFuture<'static, Result<usize>>),
}

pub struct QuicStream {
    state: Arc<RawQuicStream>,
    poll: Option<QuicStreamPoll>,
}

/// Safety: only AsyncRead/AsyncWrite functions will access the `poll` field.
unsafe impl Sync for QuicStream {}

impl Clone for QuicStream {
    fn clone(&self) -> Self {
        assert!(!self.poll.is_none(),"Clone quic stream, after returns pending by calling AsyncWrite / AsyncRead poll function.");

        Self {
            state: self.state.clone(),
            poll: None,
        }
    }
}

impl QuicStream {
    fn new(stream_id: u64, conn: QuicConnState) -> Self {
        Self {
            state: Arc::new(RawQuicStream { stream_id, conn }),
            poll: None,
        }
    }
}

impl QuicStream {
    /// Returns current stream id value.
    pub fn id(&self) -> u64 {
        self.state.stream_id
    }

    /// Returns this stream's source connection id.
    pub fn scid(&self) -> &ConnectionId<'_> {
        &self.state.conn.id
    }

    /// Writes data to a stream.
    ///
    /// On success the number of bytes written is returned.
    pub async fn send<Buf: AsRef<[u8]>>(&self, buf: Buf, fin: bool) -> Result<usize> {
        self.state.send(buf, fin).await
    }

    /// Reads contiguous data from a stream into the provided slice.
    ///
    /// The slice must be sized by the caller and will be populated up to its capacity.
    /// On success the amount of bytes read and a flag indicating the fin state is
    /// returned as a tuple.
    ///
    /// Reading data from a stream may trigger queueing of control messages
    /// (e.g. MAX_STREAM_DATA). send() should be called after reading.
    pub async fn recv<Buf: AsMut<[u8]>>(&self, buf: Buf) -> Result<(usize, bool)> {
        self.state.recv(buf).await
    }
}

impl AsyncWrite for QuicStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize>> {
        let mut fut = if let Some(QuicStreamPoll::PollWrite(fut)) = self.poll.take() {
            fut
        } else {
            Box::pin(self.state.clone().send_owned(buf.to_owned(), false))
        };

        match fut.poll_unpin(cx) {
            Poll::Pending => {
                self.poll = Some(QuicStreamPoll::PollWrite(fut));

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
        let mut fut = if let Some(QuicStreamPoll::PollClose(fut)) = self.poll.take() {
            fut
        } else {
            Box::pin(self.state.clone().send_owned(vec![], false))
        };

        match fut.poll_unpin(cx) {
            Poll::Pending => {
                self.poll = Some(QuicStreamPoll::PollClose(fut));

                Poll::Pending
            }

            Poll::Ready(r) => Poll::Ready(r.map(|_| ())),
        }
    }
}

impl AsyncRead for QuicStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<Result<usize>> {
        let mut fut = if let Some(QuicStreamPoll::PollRead(fut)) = self.poll.take() {
            fut
        } else {
            Box::pin(self.state.clone().recv_owned(buf.len()))
        };

        match fut.poll_unpin(cx) {
            Poll::Ready(Ok((packet, _))) => {
                buf[..packet.len()].copy_from_slice(&packet);

                Poll::Ready(Ok(packet.len()))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => {
                self.poll = Some(QuicStreamPoll::PollRead(fut));
                Poll::Pending
            }
        }
    }
}
