use std::{
    collections::{HashMap, VecDeque},
    io,
    net::SocketAddr,
    sync::Arc,
    task::Poll,
};

use bytes::Bytes;
use quiche::{ConnectionId, RecvInfo, SendInfo};
use rasi::futures::lock::Mutex;
use rasi::futures::{FutureExt, Stream, StreamExt};
use ring::{hmac::Key, rand::SystemRandom};

use crate::{
    future::{
        batching,
        event_map::{EventMap, EventStatus},
    },
    net::quic::errors::map_quic_error,
    utils::ReadBuf,
};

use super::{Config, QuicConnState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum QuicListenerEvent {
    /// Newly incoming connection event.
    Accept,
}

#[allow(unused)]
enum QuicListenerHandshake {
    Connection {
        conn_state: QuicConnState,
        is_established: bool,
        /// the number of bytes processed from the input buffer
        read_size: usize,
    },
    Response {
        /// buf of response packet.
        buf: Bytes,
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
            buf: read_buf.into_bytes(Some(write_size)),
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
            buf: read_buf.into_bytes(Some(write_size)),
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

/// The stream of the output data of [`QuicListenerState`]
pub struct QuicListenerStateOutputStream {
    max_send_udp_payload_size: usize,
    send_group: batching::Group<(QuicConnState, io::Result<(Bytes, SendInfo)>)>,
    send_ready: batching::Ready<(QuicConnState, io::Result<(Bytes, SendInfo)>)>,
}

impl QuicListenerStateOutputStream {
    async fn send(
        state: QuicConnState,
        max_send_udp_payload_size: usize,
    ) -> (QuicConnState, io::Result<(Bytes, SendInfo)>) {
        let mut buf = ReadBuf::with_capacity(max_send_udp_payload_size);

        match state.send(buf.chunk_mut()).await {
            Ok((send_size, send_info)) => {
                return (state, Ok((buf.into_bytes(Some(send_size)), send_info)));
            }
            Err(err) => return (state, Err(err)),
        }
    }
}

impl Stream for QuicListenerStateOutputStream {
    type Item = io::Result<(Bytes, SendInfo)>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.send_ready.poll_next_unpin(cx) {
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some((cx, r))) => {
                self.send_group
                    .join(Self::send(cx, self.max_send_udp_payload_size));

                Poll::Ready(Some(r))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// The stream of the newly incoming connections of the [`QuicListenerState`]
pub struct QuicListenerStateIncoming {
    /// raw state machine protected by mutex.
    raw: Arc<Mutex<RawQuicListenerState>>,
    /// the event center of this quic server listener.
    event_map: Arc<EventMap<QuicListenerEvent>>,
}

impl QuicListenerStateIncoming {
    pub async fn accept(&self) -> Option<QuicConnState> {
        loop {
            let mut raw = self.raw.lock().await;
            if let Some(stream_id) = raw.incoming_conns.pop_front() {
                return Some(stream_id);
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

impl Stream for QuicListenerStateIncoming {
    type Item = QuicConnState;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Box::pin(self.accept()).poll_unpin(cx)
    }
}

/// The state machine of quic server listener.
pub struct QuicListenerState {
    /// raw state machine protected by mutex.
    raw: Arc<Mutex<RawQuicListenerState>>,
    /// the event center of this quic server listener.
    event_map: Arc<EventMap<QuicListenerEvent>>,
    /// batch read group.
    send_group: batching::Group<(QuicConnState, io::Result<(Bytes, SendInfo)>)>,
}

impl QuicListenerState {
    /// Create new `QuicListenerState` instance with provided [`Config`]
    pub fn new(
        config: Config,
    ) -> io::Result<(
        Self,
        QuicListenerStateIncoming,
        QuicListenerStateOutputStream,
    )> {
        let max_send_udp_payload_size = config.max_send_udp_payload_size;
        let (send_group, send_ready) = batching::Group::new();

        let raw = Arc::new(Mutex::new(RawQuicListenerState::new(config)?));

        let event_map: Arc<EventMap<QuicListenerEvent>> = Default::default();

        Ok((
            Self {
                raw: raw.clone(),
                event_map: event_map.clone(),
                send_group: send_group.clone(),
            },
            QuicListenerStateIncoming { raw, event_map },
            QuicListenerStateOutputStream {
                max_send_udp_payload_size,
                send_group,
                send_ready,
            },
        ))
    }

    /// Processes QUIC packets received from the client.
    ///
    /// On success , returns the number of bytes processed from the input buffer and the optional response data.
    pub async fn recv(
        &self,
        buf: &mut [u8],
        recv_info: RecvInfo,
    ) -> io::Result<(usize, Option<Bytes>)> {
        let header =
            quiche::Header::from_slice(buf, quiche::MAX_CONN_ID_LEN).map_err(map_quic_error)?;

        let mut raw = self.raw.lock().await;

        if let Some((conn, is_established)) = raw.get_conn(&header.dcid) {
            // release the lock before call [QuicConnState::recv] function.
            drop(raw);

            let recv_size = conn.recv(buf, recv_info).await?;

            if !is_established && conn.is_established().await {
                // relock the state.
                raw = self.raw.lock().await;
                // move the connection to established set and push state into incoming queue.
                raw.established(&header.dcid);

                self.event_map
                    .notify(QuicListenerEvent::Accept, EventStatus::Ready);
            }

            return Ok((recv_size, None));
        }

        // Perform the handshake process.
        match raw.handshake(&header, buf, recv_info)? {
            QuicListenerHandshake::Connection {
                conn_state,
                is_established: _,
                read_size,
            } => {
                // add conn_state into batch send group.
                self.send_group.join(QuicListenerStateOutputStream::send(
                    conn_state,
                    raw.config.max_send_udp_payload_size,
                ));

                return Ok((read_size, None));
            }
            QuicListenerHandshake::Response {
                buf,
                read_size: recv_size,
            } => return Ok((recv_size, Some(buf))),
        }
    }
}
