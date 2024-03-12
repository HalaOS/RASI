use std::{
    collections::{HashSet, VecDeque},
    fmt::{Debug, Display},
    io,
    net::{SocketAddr, ToSocketAddrs},
    ops::{self, DerefMut},
    sync::Arc,
    time::{Duration, Instant},
};

// use hala_sync::{AsyncLockable, AsyncSpinMutex};
use quiche::{ConnectionId, RecvInfo, SendInfo, Shutdown};
use rand::{seq::IteratorRandom, thread_rng};
use rasi::{executor::spawn, syscall::global_network, time::TimeoutExt};

use futures::{AsyncRead, AsyncWrite, FutureExt, TryStreamExt};

use ring::rand::{SecureRandom, SystemRandom};

use crate::{
    future::event_map::{EventMap, EventStatus},
    net::{
        quic::errors::map_event_map_error,
        udp_group::{self, PathInfo, UdpGroup},
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
    /// This event notify that peer_streams_left_bidi > 0
    CanOpenPeerStream(ConnectionId<'static>),
}

/// The inner state machine for quic connection.
struct RawQuicConnState {
    /// The underlying quiche [`Connection`](quiche::Connection) instance.
    quiche_conn: quiche::Connection,
    /// The point in time when the most recent ack packet was sent.
    send_ack_eliciting_instant: Instant,
    /// The time interval for sending ping packets.
    ping_send_intervals: Option<Duration>,
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
        ping_send_intervals: Option<Duration>,
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
    /// `server_name` parameter is used to verify the peer's certificate.
    pub fn new_client(
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
    pub fn new(quiche_conn: quiche::Connection, ping_send_intervals: Option<Duration>) -> Self {
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

    fn handle_stream_status<Guard>(&self, raw: &mut Guard)
    where
        Guard: DerefMut<Target = RawQuicConnState>,
    {
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

        if raw.quiche_conn.peer_streams_left_bidi() > raw.outbound_stream_ids.len() as u64 {
            raised_events.push(QuicConnStateEvent::CanOpenPeerStream(self.scid.clone()));
        }

        self.event_map
            .notify_all(&raised_events, EventStatus::Ready);
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

                    // On success, the streams status may changed.
                    // check readable / writable status and trigger events.
                    self.handle_stream_status(&mut raw);

                    return Ok((send_size, send_info));
                }
                Err(quiche::Error::Done) => {
                    // check if the connect status `is_draining`
                    // if raw.quiche_conn.is_draining() {
                    //     return Err(io::Error::new(
                    //         io::ErrorKind::BrokenPipe,
                    //         format!("{} is draining.", self),
                    //     ));
                    // }

                    if let Some(ping_send_intervals) = raw.ping_send_intervals {
                        if raw.send_ack_eliciting_instant.elapsed() >= ping_send_intervals {
                            raw.quiche_conn
                                .send_ack_eliciting()
                                .map_err(map_quic_error)?;

                            // send `ack_eliciting` immediately.
                            continue;
                        }
                    }

                    let event = QuicConnStateEvent::Send(self.scid.clone());

                    if let Some(mut timeout_at) = raw.quiche_conn.timeout_instant() {
                        let mut send_ack_eliciting = false;

                        if let Some(ping_send_intervals) = raw.ping_send_intervals {
                            let send_ack_eliciting_at =
                                raw.send_ack_eliciting_instant + ping_send_intervals;

                            if send_ack_eliciting_at < timeout_at {
                                timeout_at = send_ack_eliciting_at;
                                send_ack_eliciting = true;
                            }
                        }

                        match self.event_map.once(event, raw).timeout_at(timeout_at).await {
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

                                if send_ack_eliciting {
                                    // send ping packet.
                                    raw.quiche_conn
                                        .send_ack_eliciting()
                                        .map_err(map_quic_error)?;
                                } else {
                                    // timeout, call quic connection timeout.
                                    raw.quiche_conn.on_timeout();
                                }

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
                Err(err) => {
                    return Err(map_quic_error(err));
                }
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

                // May received CONNECTION_CLOSE frm.
                raw.validate_conn_status(&self.event_map)?;

                // reset the `send_ack_eliciting_instant`
                raw.send_ack_eliciting_instant = Instant::now();

                // On success, the streams status may changed.
                // check readable / writable status and trigger events.

                self.handle_stream_status(&mut raw);

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
            raw.validate_conn_status(&self.event_map)?;

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
                Err(quiche::Error::InvalidStreamState(_)) => {
                    // the stream is not created yet.
                    if raw.outbound_stream_ids.contains(&stream_id) {
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
                    } else {
                        return Err(map_quic_error(quiche::Error::InvalidStreamState(stream_id)));
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
            raw.validate_conn_status(&self.event_map)?;

            let stream_send = raw.quiche_conn.stream_send(stream_id, buf, fin);

            // After calling stream_send,
            // The [`peer_streams_left_bidi`](https://docs.rs/quiche/latest/quiche/struct.Connection.html#method.peer_streams_left_bidi)
            // will be able to return the correct value.
            //
            // So the outbound stream id can be removed from `outbound_stream_ids`, safely.
            if stream_id % 2 == raw.next_outbound_stream_id % 2 {
                // notify can open next stream.
                if raw.outbound_stream_ids.remove(&stream_id) {
                    self.event_map.notify(
                        QuicConnStateEvent::CanOpenPeerStream(self.scid.clone()),
                        EventStatus::Ready,
                    );
                }
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
                Err(quiche::Error::Done) => {}
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

        // clear buf datas.
        raw.inbound_stream_ids.remove(&stream_id);
        raw.outbound_stream_ids.remove(&stream_id);

        self.event_map.notify_all(
            &[
                QuicConnStateEvent::StreamReadable(self.scid.clone(), stream_id),
                QuicConnStateEvent::StreamWritable(self.scid.clone(), stream_id),
            ],
            EventStatus::Cancel,
        );

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

            if raw.validate_conn_status(&self.event_map).is_err() {
                return None;
            }

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
    ///
    /// If `stream_limits_error` is true, this function will raise `StreamLimits` error,
    /// Otherwise it blocks the current task until a new data stream can be opened.
    pub async fn stream_open(&self, stream_limits_error: bool) -> io::Result<u64> {
        loop {
            let mut raw = self.raw.lock().await;

            raw.validate_conn_status(&self.event_map)?;

            let peer_streams_left_bidi = Self::peer_streams_left_bidi_inner(&raw);

            if peer_streams_left_bidi == 0 {
                if stream_limits_error {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        quiche::Error::StreamLimit,
                    ));
                }

                log::trace!("{} stream open pending...", self);

                self.event_map
                    .once(
                        QuicConnStateEvent::CanOpenPeerStream(self.scid.clone()),
                        raw,
                    )
                    .await
                    .map_err(map_event_map_error)?;

                continue;
            }

            // Safety: the next_outbound_stream_id >= 4
            // let prev_stream_id = raw.next_outbound_stream_id - 4;

            // if raw.outbound_stream_ids.contains(&prev_stream_id) {
            //     self.event_map
            //         .once(
            //             QuicConnStateEvent::CanOpenPeerStream(self.scid.clone()),
            //             raw,
            //         )
            //         .await
            //         .map_err(map_event_map_error)?;

            //     continue;
            // }

            let stream_id = raw.next_outbound_stream_id;

            raw.next_outbound_stream_id += 4;

            // removed after first call to stream_send.
            raw.outbound_stream_ids.insert(stream_id);

            return Ok(stream_id);
        }
    }

    /// Returns the number of bidirectional streams that can be created
    /// before the peer's stream count limit is reached.
    ///
    /// This can be useful to know if it's possible to create a bidirectional
    /// stream without trying it first.
    pub async fn peer_streams_left_bidi(&self) -> u64 {
        let raw = self.raw.lock().await;

        Self::peer_streams_left_bidi_inner(&raw)
    }

    fn peer_streams_left_bidi_inner<Guard>(raw: &Guard) -> u64
    where
        Guard: DerefMut<Target = RawQuicConnState>,
    {
        let peer_streams_left_bidi = raw.quiche_conn.peer_streams_left_bidi();
        let outgoing_cached = raw.outbound_stream_ids.len() as u64;
        let initial_max_streams_bidi = raw
            .quiche_conn
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

    /// Returns true if the connection handshake is complete.
    pub async fn is_established(&self) -> bool {
        let raw = self.raw.lock().await;
        raw.quiche_conn.is_established()
    }

    /// Returns true if the connection is closed.
    pub async fn is_closed(&self) -> bool {
        let raw = self.raw.lock().await;
        raw.quiche_conn.is_closed()
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

    async fn update_dcid(&mut self) {
        self.dcid = self
            .raw
            .lock()
            .await
            .quiche_conn
            .destination_id()
            .clone()
            .into_owned();
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
                    log::error!("drop with error: {}", err);
                }
            }
        });
    }
}

/// A builder for client side [`QuicConn`].
pub struct QuicConnector {
    udp_group: UdpGroup,
    conn_state: QuicConnState,
    max_send_udp_payload_size: usize,
}

impl QuicConnector {
    /// Create new `QuicConnector` instance with global [`syscall`](rasi::syscall::Network),
    /// to create a new Quic connection connected to the specified addresses.
    ///
    /// see [`new_with`](Self::new_with) for more informations.
    pub async fn new<L: ToSocketAddrs, R: ToSocketAddrs>(
        server_name: Option<&str>,
        laddrs: L,
        raddrs: R,
        config: &mut Config,
    ) -> io::Result<Self> {
        Self::new_with(server_name, laddrs, raddrs, config, global_network()).await
    }
    /// Create new `QuicConnector` instance with custom [`syscall`](rasi::syscall::Network),
    /// to create a new Quic connection connected to the specified addresses.
    pub async fn new_with<L: ToSocketAddrs, R: ToSocketAddrs>(
        server_name: Option<&str>,
        laddrs: L,
        raddrs: R,
        config: &mut Config,
        syscall: &'static dyn rasi::syscall::Network,
    ) -> io::Result<Self> {
        let udp_group = UdpGroup::bind_with(laddrs, syscall).await?;

        let raddr = raddrs.to_socket_addrs()?.choose(&mut thread_rng()).unwrap();

        let laddr = udp_group
            .local_addrs()
            .filter(|addr| raddr.is_ipv4() == addr.is_ipv4())
            .choose(&mut thread_rng())
            .unwrap();

        let conn_state = QuicConnState::new_client(server_name, *laddr, raddr, config)?;

        Ok(Self {
            conn_state,
            udp_group,
            max_send_udp_payload_size: config.max_send_udp_payload_size,
        })
    }

    /// Performs a real connection process.
    pub async fn connect(mut self) -> io::Result<QuicConn> {
        let (sender, mut receiver) = self.udp_group.split();

        loop {
            let mut read_buf = ReadBuf::with_capacity(self.max_send_udp_payload_size);

            let (read_size, send_info) = self.conn_state.send(read_buf.chunk_mut()).await?;

            let send_size = sender
                .send_to_on_path(
                    &read_buf.into_bytes(Some(read_size)),
                    PathInfo {
                        from: send_info.from,
                        to: send_info.to,
                    },
                )
                .await?;

            log::trace!("Quic connection, {:?}, send data {}", send_info, send_size);

            let (mut buf, path_info) =
                if let Some(timeout_at) = self.conn_state.to_inner_conn().await.timeout_instant() {
                    match receiver.try_next().timeout_at(timeout_at).await {
                        Some(Ok(r)) => r.ok_or(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "Underlying udp socket closed",
                        ))?,
                        Some(Err(err)) => {
                            return Err(err);
                        }
                        None => {
                            continue;
                        }
                    }
                } else {
                    receiver.try_next().await?.ok_or(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "Underlying udp socket closed",
                    ))?
                };

            log::trace!("Quic connection, {:?}, recv data {}", path_info, buf.len());

            self.conn_state
                .recv(
                    &mut buf,
                    RecvInfo {
                        from: path_info.from,
                        to: path_info.to,
                    },
                )
                .await?;

            if self.conn_state.is_established().await {
                self.conn_state.update_dcid().await;
                break;
            }
        }

        spawn(QuicConn::recv_loop(self.conn_state.clone(), receiver));
        spawn(QuicConn::send_loop(
            self.conn_state.clone(),
            sender,
            self.max_send_udp_payload_size,
        ));

        Ok(QuicConn::new(self.conn_state))
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

impl Display for QuicConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.0)
    }
}

impl QuicConn {
    /// Returns the source connection ID.
    ///
    /// When there are multiple IDs, and if there is an active path, the ID used
    /// on that path is returned. Otherwise the oldest ID is returned.
    ///
    /// Note that the value returned can change throughout the connection's
    /// lifetime.
    pub fn source_id(&self) -> &ConnectionId<'static> {
        &self.inner.0.scid
    }

    /// Create `QuicConn` instance from [`state`](QuicConnState).
    pub(super) fn new(state: QuicConnState) -> QuicConn {
        Self {
            inner: Arc::new(QuicConnFinalizer(state)),
        }
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
    ///
    /// If `stream_limits_error` is true, this function will raise `StreamLimits` error,
    /// Otherwise it blocks the current task until a new data stream can be opened.
    pub async fn stream_open(&self, stream_limits_error: bool) -> io::Result<QuicStream> {
        self.inner
            .0
            .stream_open(stream_limits_error)
            .await
            .map(|stream_id| QuicStream::new(stream_id, self.inner.clone()))
    }

    /// Returns the number of bidirectional streams that can be created
    /// before the peer's stream count limit is reached.
    ///
    /// This can be useful to know if it's possible to create a bidirectional
    /// stream without trying it first.
    pub async fn peer_streams_left_bidi(&self) -> u64 {
        self.inner.0.peer_streams_left_bidi().await
    }

    /// Get reference of inner [`quiche::Connection`] type.
    pub async fn to_inner_conn(&self) -> impl ops::Deref<Target = quiche::Connection> + '_ {
        self.inner.0.to_inner_conn().await
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
            log::trace!("QuicConn {:?}, recv data {}", path_info, buf.len());

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

    pub(crate) async fn send_loop(
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
        sender: udp_group::Sender,
        max_send_udp_payload_size: usize,
    ) -> io::Result<()> {
        loop {
            let mut read_buf = ReadBuf::with_capacity(max_send_udp_payload_size);

            let (send_size, send_info) = state.send(read_buf.chunk_mut()).await?;

            let send_size = sender
                .send_to_on_path(
                    &read_buf.into_bytes(Some(send_size)),
                    PathInfo {
                        from: send_info.from,
                        to: send_info.to,
                    },
                )
                .await?;

            log::trace!("QuicConn {:?}, send data {}", send_info, send_size);
        }
    }
}

struct QuicStreamFinalizer(u64, Arc<QuicConnFinalizer>);

impl Drop for QuicStreamFinalizer {
    fn drop(&mut self) {
        let state = self.1 .0.clone();
        let stream_id = self.0;

        spawn(async move {
            match state.stream_close(stream_id).await {
                Ok(_) => {}
                Err(err) => {
                    log::error!("drop with error: {}", err);
                }
            }
        });
    }
}

/// A Quic stream between a local and a remote socket.
#[derive(Clone)]
pub struct QuicStream {
    inner: Arc<QuicStreamFinalizer>,
}

impl Display for QuicStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, stream_id={}", self.inner.1 .0, self.inner.0)
    }
}

impl Debug for QuicStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicStream")
            .field("stream_id", &self.inner.0)
            .field("conn", &self.inner.1 .0.to_string())
            .finish()
    }
}

impl QuicStream {
    fn new(stream_id: u64, inner: Arc<QuicConnFinalizer>) -> Self {
        Self {
            inner: Arc::new(QuicStreamFinalizer(stream_id, inner)),
        }
    }

    /// Writes data to a stream.
    ///
    /// On success the number of bytes written is returned.
    ///
    /// Unlike the [`AsyncWrite`], this function you can manual set fin flag.
    pub async fn stream_send(&self, buf: &[u8], fin: bool) -> io::Result<usize> {
        self.inner.1 .0.stream_send(self.inner.0, buf, fin).await
    }

    /// Reads contiguous data from a stream into the provided slice.
    ///
    /// On success, returns the number of bytes written and fin flag.
    ///
    pub async fn stream_recv(&self, buf: &mut [u8]) -> io::Result<(usize, bool)> {
        self.inner.1 .0.stream_recv(self.inner.0, buf).await
    }
}

impl AsyncWrite for QuicStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        Box::pin(self.inner.1 .0.stream_send(self.inner.0, buf, false)).poll_unpin(cx)
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
        Box::pin(self.inner.1 .0.stream_close(self.inner.0))
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
        Box::pin(self.inner.1 .0.stream_recv(self.inner.0, buf))
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
