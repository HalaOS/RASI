//! This mod defined the quic protocol inner state machines.
//!
//! This is a low-level api for quic and should not be used directly.
//!
//! You should ***Use rasi compatible api *** as default.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Display,
    io,
    net::SocketAddr,
    ops::{self, DerefMut},
    sync::Arc,
    time::{Duration, Instant},
};

use bytes::BytesMut;
use quiche::{ConnectionId, RecvInfo, SendInfo, Shutdown};
use rasi::future::FutureExt;
use rasi::stream::Stream;
use rasi::time::TimeoutExt;
use ring::{
    hmac::Key,
    rand::{SecureRandom, SystemRandom},
};

use crate::{
    future::event_map::{EventMap, EventStatus},
    utils::{AsyncLockable, AsyncSpinMutex, DerefExt, ReadBuf},
};

use super::{
    errors::{map_event_map_error, map_quic_error},
    Config,
};

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

    fn can_recv_send(&self, event_map: &EventMap<QuicConnStateEvent>) -> io::Result<()> {
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

    fn can_stream_recv_send(&self) -> io::Result<()> {
        if self.quiche_conn.is_closed()
            || self.quiche_conn.is_draining()
            || self.quiche_conn.is_timed_out()
        {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                format!("{} is draining or closed", self),
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
        let next_outbound_stream_id = if quiche_conn.is_server() { 1 } else { 0 };

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
            raw.can_recv_send(&self.event_map)?;

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
                raw.can_recv_send(&self.event_map)?;

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
            raw.can_stream_recv_send()?;

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
            raw.can_stream_recv_send()?;

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

            if raw.can_recv_send(&self.event_map).is_err() {
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

            raw.can_stream_recv_send()?;

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

    pub(super) async fn update_dcid(&mut self) {
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

enum QuicListenerHandshake {
    Connection {
        #[allow(unused)]
        conn_state: QuicConnState,
        is_established: bool,
        /// the number of bytes processed from the input buffer
        read_size: usize,
    },
    Response {
        /// buf of response packet.
        buf: BytesMut,
        /// the number of bytes processed from the input buffer
        read_size: usize,
    },
}

/// Internal state machine of [`QuicListener`]
#[allow(unused)]
struct RawQuicListenerState {
    /// The quic config shared between connections for this listener.
    config: Config,
    /// The seed for source id generation .
    scid_seed: Key,
    /// Collection of quic connections in handshaking state
    handshaking_conns: HashMap<ConnectionId<'static>, QuicConnState>,
    /// Collection of established connections.
    established_conns: HashMap<ConnectionId<'static>, QuicConnState>,
    /// The fifo queue of new incoming established connections.
    incoming_conns: VecDeque<QuicConnState>,
}

#[allow(unused)]
impl RawQuicListenerState {
    /// Create `RawQuicListenerState` quic connection config.
    fn new(config: Config) -> io::Result<Self> {
        let rng = SystemRandom::new();

        let scid_seed = ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rng)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{}", err)))?;

        Ok(Self {
            config,
            scid_seed,
            handshaking_conns: Default::default(),
            established_conns: Default::default(),
            incoming_conns: Default::default(),
        })
    }

    /// Get connection by id.
    ///
    /// If found, returns tuple (QuicConnState, is_established).
    fn get_conn<'a>(&self, id: &ConnectionId<'a>) -> Option<(QuicConnState, bool)> {
        if let Some(conn) = self.handshaking_conns.get(id) {
            return Some((conn.clone(), false));
        }

        if let Some(conn) = self.established_conns.get(id) {
            return Some((conn.clone(), true));
        }

        None
    }

    /// Move connection from handshaking set to established set by id.
    fn established<'a>(&mut self, id: &ConnectionId<'a>) {
        let id = id.clone().into_owned();
        if let Some(conn) = self.handshaking_conns.remove(&id) {
            self.established_conns.insert(id, conn.clone());
            self.incoming_conns.push_back(conn);
        }
    }

    /// remove connection from pool.
    fn remove_conn<'a>(&mut self, id: &ConnectionId<'a>) -> bool {
        let id = id.clone().into_owned();
        if self.handshaking_conns.remove(&id).is_some() {
            return true;
        }

        if self.established_conns.remove(&id).is_some() {
            return true;
        }

        false
    }

    /// Process Initial packet.
    fn handshake<'a>(
        &mut self,
        header: &quiche::Header<'a>,
        buf: &'a mut [u8],
        recv_info: RecvInfo,
    ) -> io::Result<QuicListenerHandshake> {
        if header.ty != quiche::Type::Initial {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid packet: {:?}", recv_info),
            ));
        }

        self.client_hello(header, buf, recv_info)
    }

    fn client_hello<'a>(
        &mut self,
        header: &quiche::Header<'a>,
        buf: &'a mut [u8],
        recv_info: RecvInfo,
    ) -> io::Result<QuicListenerHandshake> {
        if !quiche::version_is_supported(header.version) {
            return self.negotiation_version(header, recv_info, buf);
        }

        let token = header.token.as_ref().unwrap();

        // generate new token and retry
        if token.is_empty() {
            return self.retry(header, recv_info, buf);
        }

        // check token .
        let odcid = Self::validate_token(token, &recv_info.from)?;

        let scid: quiche::ConnectionId<'_> = header.dcid.clone();

        if quiche::MAX_CONN_ID_LEN != scid.len() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                format!("Check dcid length error, len={}", scid.len()),
            ));
        }

        let mut quiche_conn = quiche::accept(
            &scid,
            Some(&odcid),
            recv_info.to,
            recv_info.from,
            &mut self.config,
        )
        .map_err(map_quic_error)?;

        let read_size = quiche_conn.recv(buf, recv_info).map_err(map_quic_error)?;

        log::trace!(
            "Create new incoming conn, scid={:?}, dcid={:?}, read_size={}",
            quiche_conn.source_id(),
            quiche_conn.destination_id(),
            read_size,
        );

        let is_established = quiche_conn.is_established();

        let conn_state = QuicConnState::new(quiche_conn, self.config.ping_packet_send_interval);

        let scid = conn_state.scid.clone();

        if is_established {
            self.established_conns.insert(scid, conn_state.clone());
            self.incoming_conns.push_back(conn_state.clone());
        } else {
            self.handshaking_conns.insert(scid, conn_state.clone());
        }

        Ok(QuicListenerHandshake::Connection {
            conn_state,
            is_established,
            read_size,
        })
    }

    fn negotiation_version<'a>(
        &mut self,
        header: &quiche::Header<'a>,
        recv_info: RecvInfo,
        buf: &mut [u8],
    ) -> io::Result<QuicListenerHandshake> {
        let scid = header.scid.clone().into_owned();
        let dcid = header.dcid.clone().into_owned();

        let mut read_buf = ReadBuf::with_capacity(self.config.max_send_udp_payload_size);

        let write_size = quiche::negotiate_version(&scid, &dcid, buf).map_err(map_quic_error)?;

        Ok(QuicListenerHandshake::Response {
            buf: read_buf.into_bytes_mut(Some(write_size)),
            read_size: buf.len(),
        })
    }
    /// Generate retry package
    fn retry<'a>(
        &mut self,
        header: &quiche::Header<'a>,
        recv_info: RecvInfo,
        buf: &mut [u8],
    ) -> io::Result<QuicListenerHandshake> {
        let token = self.mint_token(&header, &recv_info.from);

        let new_scid = ring::hmac::sign(&self.scid_seed, &header.dcid);
        let new_scid = &new_scid.as_ref()[..quiche::MAX_CONN_ID_LEN];
        let new_scid = quiche::ConnectionId::from_vec(new_scid.to_vec());

        let scid = header.scid.clone().into_owned();
        let dcid: ConnectionId<'_> = header.dcid.clone().into_owned();
        let version = header.version;

        let mut read_buf = ReadBuf::with_capacity(self.config.max_send_udp_payload_size);

        let write_size = quiche::retry(
            &scid,
            &dcid,
            &new_scid,
            &token,
            version,
            read_buf.chunk_mut(),
        )
        .map_err(map_quic_error)?;

        Ok(QuicListenerHandshake::Response {
            buf: read_buf.into_bytes_mut(Some(write_size)),
            read_size: buf.len(),
        })
    }

    fn validate_token<'a>(
        token: &'a [u8],
        src: &SocketAddr,
    ) -> io::Result<quiche::ConnectionId<'a>> {
        if token.len() < 6 {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                format!("Invalid token, token length < 6"),
            ));
        }

        if &token[..6] != b"quiche" {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                format!("Invalid token, not start with 'quiche'"),
            ));
        }

        let token = &token[6..];

        let addr = match src.ip() {
            std::net::IpAddr::V4(a) => a.octets().to_vec(),
            std::net::IpAddr::V6(a) => a.octets().to_vec(),
        };

        if token.len() < addr.len() || &token[..addr.len()] != addr.as_slice() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                format!("Invalid token, address mismatch"),
            ));
        }

        Ok(quiche::ConnectionId::from_ref(&token[addr.len()..]))
    }

    fn mint_token<'a>(&self, hdr: &quiche::Header<'a>, src: &SocketAddr) -> Vec<u8> {
        let mut token = Vec::new();

        token.extend_from_slice(b"quiche");

        let addr = match src.ip() {
            std::net::IpAddr::V4(a) => a.octets().to_vec(),
            std::net::IpAddr::V6(a) => a.octets().to_vec(),
        };

        token.extend_from_slice(&addr);
        token.extend_from_slice(&hdr.dcid);

        token
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum QuicListenerEvent {
    /// Newly incoming connection event.
    Accept,
}

/// The stream of the newly incoming connections of the [`QuicListenerState`].
pub struct QuicServerStateIncoming {
    /// raw state machine protected by mutex.
    raw: Arc<AsyncSpinMutex<RawQuicListenerState>>,
    /// the event center of this quic server listener.
    event_map: Arc<EventMap<QuicListenerEvent>>,
}

impl QuicServerStateIncoming {
    pub async fn accept(&self) -> Option<QuicConnState> {
        loop {
            let mut raw = self.raw.lock().await;
            if let Some(conn_state) = raw.incoming_conns.pop_front() {
                return Some(conn_state);
            }

            match self.event_map.once(QuicListenerEvent::Accept, raw).await {
                Ok(_) => {}
                Err(err) => {
                    log::error!("cancel quic server accept loop with error: {:?}", err);
                    return None;
                }
            }
        }
    }
}

impl Stream for QuicServerStateIncoming {
    type Item = QuicConnState;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Box::pin(self.accept()).poll_unpin(cx)
    }
}

/// The state machine of quic server listener.
#[derive(Clone)]
pub struct QuicListenerState {
    /// raw state machine protected by mutex.
    raw: Arc<AsyncSpinMutex<RawQuicListenerState>>,
    /// the event center of this quic server listener.
    event_map: Arc<EventMap<QuicListenerEvent>>,
}

impl QuicListenerState {
    /// Create new `QuicListenerState` instance with provided [`Config`]
    pub fn new(config: Config) -> io::Result<(Self, QuicServerStateIncoming)> {
        let raw = Arc::new(AsyncSpinMutex::new(RawQuicListenerState::new(config)?));

        let event_map: Arc<EventMap<QuicListenerEvent>> = Default::default();

        Ok((
            Self {
                raw: raw.clone(),
                event_map: event_map.clone(),
            },
            QuicServerStateIncoming { raw, event_map },
        ))
    }

    /// Processes QUIC packets received from the client.
    ///
    /// On success , returns the number of bytes processed from the input buffer and the optional response data.
    pub async fn recv(
        &self,
        buf: &mut [u8],
        recv_info: RecvInfo,
    ) -> io::Result<(usize, Option<BytesMut>, Option<QuicConnState>)> {
        let header =
            quiche::Header::from_slice(buf, quiche::MAX_CONN_ID_LEN).map_err(map_quic_error)?;

        let mut raw = self.raw.lock().await;

        if let Some((conn, is_established)) = raw.get_conn(&header.dcid) {
            // release the lock before call [QuicConnState::recv] function.
            drop(raw);

            let recv_size = match conn.recv(buf, recv_info).await {
                Ok(recv_size) => recv_size,
                Err(err) => {
                    if conn.is_closed().await {
                        // relock the state.
                        raw = self.raw.lock().await;

                        raw.remove_conn(&header.dcid);

                        log::info!("{}, removed from server pool.", conn);
                    }

                    return Err(err);
                }
            };

            if !is_established && conn.is_established().await {
                // relock the state.
                raw = self.raw.lock().await;
                // move the connection to established set and push state into incoming queue.
                raw.established(&header.dcid);

                self.event_map
                    .notify(QuicListenerEvent::Accept, EventStatus::Ready);
            }

            return Ok((recv_size, None, None));
        }

        // Perform the handshake process.
        match raw.handshake(&header, buf, recv_info)? {
            QuicListenerHandshake::Connection {
                conn_state,
                is_established,
                read_size,
            } => {
                // notify incoming queue read ops.
                if is_established {
                    self.event_map
                        .notify(QuicListenerEvent::Accept, EventStatus::Ready);
                }

                return Ok((read_size, None, Some(conn_state)));
            }
            QuicListenerHandshake::Response {
                buf,
                read_size: recv_size,
            } => return Ok((recv_size, Some(buf), None)),
        }
    }

    pub async fn remove_conn(&self, scid: &ConnectionId<'static>) {
        let mut raw = self.raw.lock().await;

        if raw.remove_conn(scid) {
            log::info!("scid={:?}, remove connection from server pool", scid);
        } else {
            log::warn!(
                "scid={:?}, removed from server pool with error: not found",
                scid
            );
        }
    }
}
