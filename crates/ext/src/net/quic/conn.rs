use std::{
    collections::{HashSet, VecDeque},
    fmt::{Debug, Display},
    io,
    net::{SocketAddr, ToSocketAddrs},
    ops,
    sync::Arc,
    time::{Duration, Instant},
};

// use hala_sync::{AsyncLockable, AsyncSpinMutex};
use quiche::{ConnectionId, RecvInfo, SendInfo, Shutdown};
use rand::{seq::IteratorRandom, thread_rng};
use rasi::{executor::spawn, syscall::global_network};

use futures::{AsyncRead, AsyncWrite, FutureExt, SinkExt, TryStreamExt};

use ring::rand::{SecureRandom, SystemRandom};

use crate::{
    future::event_map::{EventMap, EventStatus},
    net::{
        quic::errors::map_event_map_error,
        udp_group::{self, UdpGroup},
    },
    utils::{AsyncLockable, AsyncSpinMutex, DerefExt, ReadBuf},
};

use super::{errors::map_quic_error, Config};

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
}

/// The inner state machine for quic connection.
struct RawQuicConnState {
    /// The underlying quiche [`Connection`](quiche::Connection) instance.
    quiche_conn: quiche::Connection,
    /// The point in time when the most recent ack packet was sent.
    send_ack_eliciting_instant: Instant,
    /// The time interval for sending ping packets.
    ping_send_intervals: Duration,
    /// Registered inbound stream ids.
    inbound_stream_ids: HashSet<u64>,
    /// Registered outbound stream ids.
    outbound_stream_ids: HashSet<u64>,
    /// Next id of outbound stream.
    next_outbound_stream_id: u64,
    /// The inbound stream acceptance queue.
    inbound_stream_queue: VecDeque<u64>,
}

impl Display for RawQuicConnState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "scid={:?}, dcid={:?}, is_server={}",
            self.quiche_conn.source_id(),
            self.quiche_conn.destination_id(),
            self.quiche_conn.is_server()
        )
    }
}

impl RawQuicConnState {
    fn new(
        quiche_conn: quiche::Connection,
        ping_send_intervals: Duration,
        next_outbound_stream_id: u64,
    ) -> Self {
        let mut this = Self {
            quiche_conn,
            send_ack_eliciting_instant: Instant::now(),
            ping_send_intervals,
            inbound_stream_ids: Default::default(),
            outbound_stream_ids: Default::default(),
            next_outbound_stream_id,
            inbound_stream_queue: Default::default(),
        };

        // handle inbound streams.
        for id in this.quiche_conn.readable() {
            if id % 2 != next_outbound_stream_id % 2 {
                this.inbound_stream_ids.insert(id);
            }

            this.inbound_stream_queue.push_back(id);
        }

        this
    }

    fn validate_conn_status(&self, event_map: &EventMap<QuicConnStateEvent>) -> io::Result<()> {
        if self.quiche_conn.is_closed() {
            event_map.close();

            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                format!("{} closed", self),
            ))
        } else {
            Ok(())
        }
    }
}

/// The inner state machine for a quic connection.
#[derive(Clone)]
pub struct QuicConnState {
    /// raw state machine protected by mutex.
    raw: Arc<AsyncSpinMutex<RawQuicConnState>>,
    /// the event center of this quic connection.
    event_map: Arc<EventMap<QuicConnStateEvent>>,
    /// The source id of this connection.
    pub scid: ConnectionId<'static>,
    /// The destination id of this connection.
    pub dcid: ConnectionId<'static>,
    /// Whether or not this is a server-side connection.
    pub is_server: bool,
}

impl Display for QuicConnState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "scid={:?}, dcid={:?}, is_server={}",
            self.scid, self.dcid, self.is_server
        )
    }
}

impl QuicConnState {
    /// Creates a new client-side connection.
    ///
    /// `server_name` parameter is used to verify the peer's
    /// certificate.
    pub fn connect(
        server_name: Option<&str>,
        laddr: SocketAddr,
        raddr: SocketAddr,
        config: &mut Config,
    ) -> io::Result<Self> {
        let mut scid = vec![0; quiche::MAX_CONN_ID_LEN];

        SystemRandom::new()
            .fill(&mut scid)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{}", err)))?;

        let scid = quiche::ConnectionId::from_vec(scid);

        let quiche_conn =
            quiche::connect(server_name, &scid, laddr, raddr, config).map_err(map_quic_error)?;

        Ok(Self::new(quiche_conn, config.ping_packet_send_interval))
    }

    /// Create new `QuicConnState` with provided parameters.
    ///
    /// # Parameters
    ///
    /// - ***quiche_conn*** The underlying quiche [`connection`](quiche::Connection) instance.
    /// - ***ping_send_intervals*** The time interval for sending ping packets.
    pub fn new(quiche_conn: quiche::Connection, ping_send_intervals: Duration) -> Self {
        let next_outbound_stream_id = if quiche_conn.is_server() { 5 } else { 4 };

        Self {
            is_server: quiche_conn.is_server(),
            scid: quiche_conn.source_id().into_owned(),
            dcid: quiche_conn.destination_id().into_owned(),
            raw: Arc::new(AsyncSpinMutex::new(RawQuicConnState::new(
                quiche_conn,
                ping_send_intervals,
                next_outbound_stream_id,
            ))),
            event_map: Arc::new(EventMap::new()),
        }
    }

    /// Read sending data from quic connection state machine.
    ///
    /// On success, returns the total number of bytes copied and the [`send information`](SendInfo)
    pub async fn send(&self, buf: &mut [u8]) -> io::Result<(usize, SendInfo)> {
        let mut raw = self.raw.lock().await;

        loop {
            // check if this connection is closed, and can be dropped.
            raw.validate_conn_status(&self.event_map)?;

            match raw.quiche_conn.send(buf) {
                Ok((send_size, send_info)) => {
                    log::trace!(
                        "{} send data, len={}, elapsed={:?}",
                        self,
                        send_size,
                        send_info.at.elapsed()
                    );

                    // reset the `send_ack_eliciting_instant`
                    raw.send_ack_eliciting_instant = Instant::now();

                    return Ok((send_size, send_info));
                }
                Err(quiche::Error::Done) => {
                    // check if the connect status `is_draining`
                    if raw.quiche_conn.is_draining() {
                        return Err(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            format!("{} is draining.", self),
                        ));
                    }

                    if raw.send_ack_eliciting_instant.elapsed() >= raw.ping_send_intervals {
                        raw.quiche_conn
                            .send_ack_eliciting()
                            .map_err(map_quic_error)?;

                        // send `ack_eliciting` immediately.
                        continue;
                    }

                    let event = QuicConnStateEvent::Send(self.scid.clone());

                    if let Some(timeout) = raw.quiche_conn.timeout_instant() {
                        use rasi::time::TimeoutExt;

                        log::trace!(
                            "{} waiting send data with timeout at {:#?}",
                            self,
                            timeout - Instant::now()
                        );

                        match self.event_map.once(event, raw).timeout_at(timeout).await {
                            Some(Ok(_)) => {
                                // try send data again.
                                raw = self.raw.lock().await;
                                continue;
                            }
                            Some(Err(err)) => {
                                // event_map report an error.
                                return Err(map_event_map_error(err));
                            }
                            None => {
                                raw = self.raw.lock().await;

                                log::trace!("{} on_timeout", self);

                                // timeout, call quic connection timeout.
                                raw.quiche_conn.on_timeout();
                                continue;
                            }
                        }
                    } else {
                        match self.event_map.once(event, raw).await {
                            Ok(_) => {
                                // try send data again.
                                raw = self.raw.lock().await;
                                continue;
                            }
                            Err(err) => {
                                // event_map report an error.
                                return Err(map_event_map_error(err));
                            }
                        }
                    }
                }
                Err(err) => return Err(map_quic_error(err)),
            }
        }
    }

    /// Processes QUIC packets received from the peer.
    ///
    /// On success the number of bytes processed from the input buffer is returned.
    pub async fn recv(&self, buf: &mut [u8], info: RecvInfo) -> io::Result<usize> {
        let mut raw = self.raw.lock().await;

        match raw.quiche_conn.recv(buf, info) {
            Ok(read_size) => {
                log::trace!("rx {}, len={}", self, read_size);

                // reset the `send_ack_eliciting_instant`
                raw.send_ack_eliciting_instant = Instant::now();

                // On success, the streams status may changed.
                // check readable / writable status and trigger events.

                let mut raised_events = vec![
                    // as [`send`](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.send)
                    // function description, that is, any time recv() is called, we should call send() again.
                    QuicConnStateEvent::Send(self.scid.clone()),
                ];

                for stream_id in raw.quiche_conn.readable() {
                    // check if the stream is a new inbound stream.
                    // If true push stream into the acceptance queue instead of triggering a readable event
                    if stream_id % 2 != raw.next_outbound_stream_id % 2
                        && !raw.inbound_stream_ids.contains(&stream_id)
                    {
                        raw.inbound_stream_ids.insert(stream_id);
                        raw.inbound_stream_queue.push_back(stream_id);
                        raised_events.push(QuicConnStateEvent::StreamAccept(self.scid.clone()));

                        log::trace!("{} stream_id={}, newly incoming stream", self, stream_id);

                        continue;
                    }

                    raised_events.push(QuicConnStateEvent::StreamReadable(
                        self.scid.clone(),
                        stream_id,
                    ));
                }

                for stream_id in raw.quiche_conn.writable() {
                    raised_events.push(QuicConnStateEvent::StreamWritable(
                        self.scid.clone(),
                        stream_id,
                    ));
                }

                self.event_map
                    .notify_all(&raised_events, EventStatus::Ready);

                Ok(read_size)
            }
            Err(err) => {
                // On error the connection will be closed by calling close() with the appropriate error code.
                // see quiche [document](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.recv)
                // for more information.

                // So we must cancel all pending listener of event_map immediately;
                self.event_map.close();

                Err(map_quic_error(err))
            }
        }
    }

    /// Reads contiguous data from a stream into the provided slice.
    ///
    /// The slice must be sized by the caller and will be populated up to its capacity.
    /// On success the amount of bytes read and a flag indicating the fin state is
    /// returned as a tuple, or Done if there is no data to read.
    ///
    /// Reading data from a stream may trigger queueing of control messages
    /// (e.g. MAX_STREAM_DATA). send() should be called after reading.
    pub async fn stream_recv(&self, stream_id: u64, buf: &mut [u8]) -> io::Result<(usize, bool)> {
        let mut raw = self.raw.lock().await;

        loop {
            match raw.quiche_conn.stream_recv(stream_id, buf) {
                Ok((read_size, fin)) => {
                    log::trace!(
                        "{}, stream_id={}, recv_len={}, fin={}",
                        self,
                        stream_id,
                        read_size,
                        fin
                    );

                    // this is the final packet, the send() should be called.
                    if fin {
                        self.event_map.notify(
                            QuicConnStateEvent::Send(self.scid.clone()),
                            EventStatus::Ready,
                        );
                    }

                    return Ok((read_size, fin));
                }
                Err(quiche::Error::Done) => {
                    // no data to read, notify send event.
                    self.event_map.notify(
                        QuicConnStateEvent::Send(self.scid.clone()),
                        EventStatus::Ready,
                    );

                    match self
                        .event_map
                        .once(
                            QuicConnStateEvent::StreamReadable(self.scid.clone(), stream_id),
                            raw,
                        )
                        .await
                    {
                        Ok(_) => {
                            raw = self.raw.lock().await;
                        }
                        Err(err) => {
                            return Err(map_event_map_error(err));
                        }
                    }
                }
                Err(err) => {
                    return Err(map_quic_error(err));
                }
            }
        }
    }

    /// Writes data to a stream.
    ///
    /// On success the number of bytes written is returned.
    pub async fn stream_send(&self, stream_id: u64, buf: &[u8], fin: bool) -> io::Result<usize> {
        let mut raw = self.raw.lock().await;

        loop {
            let stream_send = raw.quiche_conn.stream_send(stream_id, buf, fin);

            // After calling stream_send,
            // The [`peer_streams_left_bidi`](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.peer_streams_left_bidi)
            // will be able to return the correct value.
            //
            // So the outbound stream id can be removed from `outbound_stream_ids`, safely.
            if stream_id % 2 == raw.next_outbound_stream_id % 2 {
                raw.outbound_stream_ids.remove(&stream_id);
            }

            match stream_send {
                Ok(send_size) => {
                    log::trace!(
                        "{}, stream_id={}, send_len={}, fin={}",
                        self,
                        stream_id,
                        send_size,
                        fin
                    );

                    // According to the function A [`send`](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.send)
                    // document description, we should call the send function immediately.
                    self.event_map.notify(
                        QuicConnStateEvent::Send(self.scid.clone()),
                        EventStatus::Ready,
                    );

                    return Ok(send_size);
                }
                Err(quiche::Error::Done) => {
                    // if no data was written(e.g. because the stream has no capacity),
                    // call `send()` function immediately
                    self.event_map.notify(
                        QuicConnStateEvent::Send(self.scid.clone()),
                        EventStatus::Ready,
                    );

                    match self
                        .event_map
                        .once(
                            QuicConnStateEvent::StreamWritable(self.scid.clone(), stream_id),
                            raw,
                        )
                        .await
                    {
                        Ok(_) => {
                            raw = self.raw.lock().await;
                        }
                        Err(err) => {
                            return Err(map_event_map_error(err));
                        }
                    }
                }
                Err(err) => {
                    return Err(map_quic_error(err));
                }
            }
        }
    }

    /// Elegantly close the stream.
    pub async fn stream_close(&self, stream_id: u64) -> io::Result<()> {
        // Safety: ensure send fin packet.
        self.stream_send(stream_id, b"", true).await?;

        let mut raw = self.raw.lock().await;

        let stream_finished = raw.quiche_conn.stream_finished(stream_id);

        if !stream_finished {
            log::warn!(
                "{},stream_id={}, Drops stream with data still unread.",
                self,
                stream_id
            );

            // drop a stream without receiving a fin packet, so we need to notify peer to stop sending data.
            match raw
                .quiche_conn
                .stream_shutdown(stream_id, Shutdown::Read, 0)
            {
                Ok(_) => {}
                Err(err) => {
                    log::error!(
                        "{}, stream_id={}, stream_shutdown with error: {}",
                        self,
                        stream_id,
                        err
                    );
                }
            }
        }

        assert!(
            !self.event_map.notify(
                QuicConnStateEvent::StreamReadable(self.scid.clone(), stream_id),
                EventStatus::Cancel
            ),
            "Call stream_drop with stream_recv operation is pending"
        );

        assert!(
            !self.event_map.notify(
                QuicConnStateEvent::StreamWritable(self.scid.clone(), stream_id),
                EventStatus::Cancel
            ),
            "Call stream_drop with stream_send operation is pending"
        );

        // clear buf datas.

        raw.inbound_stream_ids.remove(&stream_id);

        raw.outbound_stream_ids.remove(&stream_id);

        // notify connection to send data.
        self.event_map.notify(
            QuicConnStateEvent::Send(self.scid.clone()),
            EventStatus::Ready,
        );

        Ok(())
    }

    /// Accepts a new incoming stream via this connection.
    ///
    /// Returns None, if the connection is draining or has been closed.
    pub async fn stream_accept(&self) -> Option<u64> {
        loop {
            let mut raw = self.raw.lock().await;
            if let Some(stream_id) = raw.inbound_stream_queue.pop_front() {
                return Some(stream_id);
            }

            match self
                .event_map
                .once(QuicConnStateEvent::StreamAccept(self.scid.clone()), raw)
                .await
            {
                Ok(_) => {}
                Err(err) => {
                    log::error!("{}, cancel accept loop with error: {:?}", self, err);
                    return None;
                }
            }
        }
    }

    /// Open a outbound stream. returns the new stream id.
    pub async fn stream_open(&self) -> u64 {
        let mut raw = self.raw.lock().await;

        let stream_id = raw.next_outbound_stream_id;

        raw.next_outbound_stream_id += 4;

        // removed after first call to stream_send.
        raw.outbound_stream_ids.insert(stream_id);

        stream_id
    }

    /// Returns the number of bidirectional streams that can be created
    /// before the peer's stream count limit is reached.
    ///
    /// This can be useful to know if it's possible to create a bidirectional
    /// stream without trying it first.
    pub async fn peer_streams_left_bidi(&self) -> u64 {
        let raw = self.raw.lock().await;

        raw.quiche_conn.peer_streams_left_bidi() - raw.outbound_stream_ids.len() as u64
    }

    /// Returns true if the connection handshake is complete.
    pub async fn is_established(&self) -> bool {
        let raw = self.raw.lock().await;
        raw.quiche_conn.is_established()
    }

    /// Get reference of inner [`quiche::Connection`] type.
    pub async fn to_inner_conn(&self) -> impl ops::Deref<Target = quiche::Connection> + '_ {
        self.raw.lock().await.deref_map(|state| &state.quiche_conn)
    }

    /// Closes the connection with the given error and reason.
    ///
    /// The `app` parameter specifies whether an application close should be
    /// sent to the peer. Otherwise a normal connection close is sent.
    ///
    /// If `app` is true but the connection is not in a state that is safe to
    /// send an application error (not established nor in early data), in
    /// accordance with [RFC
    /// 9000](https://www.rfc-editor.org/rfc/rfc9000.html#section-10.2.3-3), the
    /// error code is changed to APPLICATION_ERROR and the reason phrase is
    /// cleared.
    pub async fn close(&self, app: bool, err: u64, reason: &[u8]) -> io::Result<()> {
        match self.raw.lock().await.quiche_conn.close(app, err, reason) {
            Ok(_) => Ok(()),
            Err(quiche::Error::Done) => Ok(()),
            Err(err) => Err(map_quic_error(err)),
        }
    }
}

struct QuicConnFinalizer(QuicConnState);

impl Drop for QuicConnFinalizer {
    fn drop(&mut self) {
        let state = self.0.clone();
        spawn(async move {
            match state.close(false, 0, b"").await {
                Ok(_) => {}
                Err(err) => {
                    log::error!("drop {} with erro: {}", state, err);
                }
            }
        });
    }
}

/// A Quic connection between a local and a remote socket.
///
/// A `QuicConn` can either be created by connecting to an endpoint, via the [`connect`](Self::connect) method,
/// or by [accepting] a connection from a [`listener`](super::QuicListener).
///
/// You can either open a stream via the [`open_stream`](Self::open_stream) function,
/// or accept a inbound stream via the [`stream_accept`](Self::stream_accept) function
pub struct QuicConn {
    inner: Arc<QuicConnFinalizer>,
}

impl QuicConn {
    /// Create `QuicConn` instance from [`state`](QuicConnState).
    pub(super) fn new(state: QuicConnState) -> QuicConn {
        Self {
            inner: Arc::new(QuicConnFinalizer(state)),
        }
    }

    // Using global [`syscall`](rasi_syscall::Network) interface to create a new Quic
    /// connection connected to the specified addresses.
    ///
    /// see [`connect_with`](Self::connect_with) for more informations.
    pub async fn connect<L: ToSocketAddrs, R: ToSocketAddrs>(
        server_name: Option<&str>,
        laddrs: L,
        raddrs: R,
        config: &mut Config,
    ) -> io::Result<Self> {
        Self::connect_with(server_name, laddrs, raddrs, config, global_network()).await
    }

    // Using custom [`syscall`](rasi_syscall::Network) interface to create a new Quic
    /// connection connected to the specified addresses.
    ///
    /// This method will create a new Quic socket and attempt to connect it to the `raddrs`
    /// provided. The [returned future] will be resolved once the connection has successfully
    /// connected, or it will return an error if one occurs.
    ///
    pub async fn connect_with<L: ToSocketAddrs, R: ToSocketAddrs>(
        server_name: Option<&str>,
        laddrs: L,
        raddrs: R,
        config: &mut Config,
        syscall: &'static dyn rasi::syscall::Network,
    ) -> io::Result<Self> {
        let socket = UdpGroup::bind_with(laddrs, syscall).await?;

        let laddr = socket.local_addrs().choose(&mut thread_rng()).unwrap();

        let raddr = raddrs.to_socket_addrs()?.choose(&mut thread_rng()).unwrap();

        let conn_state = QuicConnState::connect(server_name, *laddr, raddr, config)?;

        let (mut sender, mut receiver) = socket.split();

        loop {
            let mut read_buf = ReadBuf::with_capacity(config.max_send_udp_payload_size);

            let (read_size, send_info) = conn_state.send(read_buf.chunk_mut()).await?;

            sender
                .send((
                    read_buf.into_bytes(Some(read_size)),
                    Some(send_info.from),
                    send_info.to,
                ))
                .await?;

            let (mut buf, path_info) = receiver.try_next().await?.ok_or(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Underlying udp socket closed",
            ))?;

            conn_state
                .recv(
                    &mut buf,
                    RecvInfo {
                        from: path_info.from,
                        to: path_info.to,
                    },
                )
                .await?;

            if conn_state.is_established().await {
                break;
            }
        }

        spawn(Self::recv_loop(conn_state.clone(), receiver));
        spawn(Self::send_loop(
            conn_state.clone(),
            sender,
            config.max_send_udp_payload_size,
        ));

        Ok(Self::new(conn_state))
    }

    /// Accepts a new incoming stream via this connection.
    ///
    /// Returns None, if the connection is draining or has been closed.
    pub async fn stream_accept(&self) -> Option<QuicStream> {
        self.inner
            .0
            .stream_accept()
            .await
            .map(|stream_id| QuicStream::new(stream_id, self.inner.clone()))
    }

    /// Open a outbound stream. returns the new stream id.
    ///
    /// The `stream_open` does not really open a stream until
    /// the first byte is sent by [`send`](QuicConnState::stream_send) function.
    pub async fn stream_open(&self) -> QuicStream {
        QuicStream::new(self.inner.0.stream_open().await, self.inner.clone())
    }
}

impl QuicConn {
    async fn recv_loop(state: QuicConnState, receiver: udp_group::Receiver) {
        match Self::recv_loop_inner(state, receiver).await {
            Ok(_) => {
                log::info!("QuicListener recv_loop stopped.");
            }
            Err(err) => {
                log::error!("QuicListener recv_loop stopped with err: {}", err);
            }
        }
    }

    async fn recv_loop_inner(
        state: QuicConnState,
        mut receiver: udp_group::Receiver,
    ) -> io::Result<()> {
        while let Some((mut buf, path_info)) = receiver.try_next().await? {
            let _ = match state
                .recv(
                    &mut buf,
                    RecvInfo {
                        from: path_info.from,
                        to: path_info.to,
                    },
                )
                .await
            {
                Ok(r) => r,
                Err(err) => {
                    log::error!(
                        "Quic client: handle packet from {:?} error: {}",
                        path_info,
                        err
                    );
                    continue;
                }
            };
        }

        Ok(())
    }

    async fn send_loop(
        state: QuicConnState,
        sender: udp_group::Sender,
        max_send_udp_payload_size: usize,
    ) {
        match Self::send_loop_inner(state, sender, max_send_udp_payload_size).await {
            Ok(_) => {
                log::info!("Quic client send_loop stopped.");
            }
            Err(err) => {
                log::error!("Quic client send_loop stopped with err: {}", err);
            }
        }
    }

    async fn send_loop_inner(
        state: QuicConnState,
        mut sender: udp_group::Sender,
        max_send_udp_payload_size: usize,
    ) -> io::Result<()> {
        loop {
            let mut read_buf = ReadBuf::with_capacity(max_send_udp_payload_size);

            let (send_size, send_info) = state.send(read_buf.chunk_mut()).await?;

            sender
                .send((
                    read_buf.into_bytes(Some(send_size)),
                    Some(send_info.from),
                    send_info.to,
                ))
                .await?;
        }
    }
}

/// A Quic stream between a local and a remote socket.
#[derive(Clone)]
pub struct QuicStream {
    stream_id: u64,
    inner: Arc<QuicConnFinalizer>,
}

impl QuicStream {
    fn new(stream_id: u64, inner: Arc<QuicConnFinalizer>) -> Self {
        Self { stream_id, inner }
    }
}

impl AsyncWrite for QuicStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        Box::pin(self.inner.0.stream_send(self.stream_id, buf, false)).poll_unpin(cx)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        Box::pin(self.inner.0.stream_close(self.stream_id))
            .poll_unpin(cx)
            .map(|_| Ok(()))
    }
}

impl AsyncRead for QuicStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        Box::pin(self.inner.0.stream_recv(self.stream_id, buf))
            .poll_unpin(cx)
            .map(|r| match r {
                Ok((readsize, _)) => Ok(readsize),
                Err(err) => {
                    if err.kind() == io::ErrorKind::BrokenPipe {
                        Ok(0)
                    } else {
                        Err(err)
                    }
                }
            })
    }
}
