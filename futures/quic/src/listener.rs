use std::collections::{HashMap, VecDeque};
use std::io::{Error, ErrorKind, Result};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use futures::lock::Mutex;
use futures::stream::unfold;
use futures::{Stream, StreamExt};
use futures_map::{FuturesWaitMap, KeyWaitMap};
use quiche::{Config, ConnectionId, RecvInfo, SendInfo};
use ring::{hmac::Key, rand::SystemRandom};

use crate::errors::map_quic_error;
use crate::{QuicConn, QuicConnState};

enum QuicListenerHandshake {
    Connection {
        #[allow(unused)]
        conn: QuicConnState,
        is_established: bool,
        /// the number of bytes processed from the input buffer
        read_size: usize,
    },
    Response {
        /// buf of response packet.
        buf: Vec<u8>,
        /// the number of bytes processed from the input buffer
        read_size: usize,
    },
}

/// Server-side incoming connection handshake pool.
pub struct QuicListenerState {
    /// The quic config shared between connections for this listener.
    config: Config,
    /// The seed for generating source id
    seed_key: Key,
    /// A quic connection pool that is in the handshake phase
    handshaking_pool: HashMap<ConnectionId<'static>, QuicConnState>,
    /// A quic connection pool that is already connected
    established_conns: HashMap<ConnectionId<'static>, QuicConnState>,
    /// When first see a inbound quic connection, push it into this queue.
    incoming_conns: VecDeque<QuicConn>,
}

impl QuicListenerState {
    /// Create `HandshakePool` with provided `config`.
    fn new(config: Config) -> Result<Self> {
        let rng = SystemRandom::new();

        let seed_key = ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rng)
            .map_err(|err| Error::new(ErrorKind::Other, format!("{}", err)))?;

        Ok(Self {
            config,
            seed_key,
            handshaking_pool: Default::default(),
            incoming_conns: Default::default(),
            established_conns: Default::default(),
        })
    }

    /// Get connection by id.
    ///
    /// If found, returns tuple (QuicConnState, is_established).
    fn get_conn<'a>(&self, id: &ConnectionId<'a>) -> Option<(QuicConnState, bool)> {
        if let Some(conn) = self.handshaking_pool.get(id) {
            return Some((conn.clone(), false));
        }

        if let Some(conn) = self.established_conns.get(id) {
            return Some((conn.clone(), true));
        }

        None
    }

    /// Move connection from handshaking set to established set by id.
    #[allow(unused)]
    fn established<'a>(&mut self, id: &ConnectionId<'a>) {
        let id = id.clone().into_owned();
        if let Some(conn) = self.handshaking_pool.remove(&id) {
            self.established_conns.insert(id, conn.clone());
            self.incoming_conns.push_back(conn.into());
        }
    }

    /// remove connection from pool.
    fn remove_conn<'a>(&mut self, id: &ConnectionId<'a>) -> bool {
        let id = id.clone().into_owned();
        if self.handshaking_pool.remove(&id).is_some() {
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
    ) -> Result<QuicListenerHandshake> {
        if header.ty != quiche::Type::Initial {
            return Err(Error::new(
                ErrorKind::InvalidData,
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
    ) -> Result<QuicListenerHandshake> {
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
            return Err(Error::new(
                ErrorKind::Interrupted,
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

        let is_established = quiche_conn.is_established();

        let scid = quiche_conn.source_id().into_owned();
        let dcid = quiche_conn.destination_id().into_owned();

        log::trace!("Create new incoming conn, scid={:?}, dcid={:?}", scid, dcid);

        let conn = QuicConnState::new(quiche_conn, 1, None);

        if is_established {
            self.established_conns.insert(scid, conn.clone());
            self.incoming_conns.push_back(conn.clone().into());
        } else {
            self.handshaking_pool.insert(scid, conn.clone());
        }

        Ok(QuicListenerHandshake::Connection {
            conn,
            is_established,
            read_size,
        })
    }

    fn negotiation_version<'a>(
        &mut self,
        header: &quiche::Header<'a>,
        _recv_info: RecvInfo,
        buf: &mut [u8],
    ) -> Result<QuicListenerHandshake> {
        let scid = header.scid.clone().into_owned();
        let dcid = header.dcid.clone().into_owned();

        let mut read_buf = vec![0; 128];

        let write_size = quiche::negotiate_version(&scid, &dcid, buf).map_err(map_quic_error)?;

        read_buf.resize(write_size, 0);

        Ok(QuicListenerHandshake::Response {
            buf: read_buf,
            read_size: buf.len(),
        })
    }
    /// Generate retry package
    fn retry<'a>(
        &mut self,
        header: &quiche::Header<'a>,
        recv_info: RecvInfo,
        buf: &mut [u8],
    ) -> Result<QuicListenerHandshake> {
        let token = self.mint_token(&header, &recv_info.from);

        let new_scid = ring::hmac::sign(&self.seed_key, &header.dcid);
        let new_scid = &new_scid.as_ref()[..quiche::MAX_CONN_ID_LEN];
        let new_scid = quiche::ConnectionId::from_vec(new_scid.to_vec());

        let scid = header.scid.clone().into_owned();
        let dcid: ConnectionId<'_> = header.dcid.clone().into_owned();
        let version = header.version;

        let mut read_buf = vec![0; 1200];

        let write_size = quiche::retry(&scid, &dcid, &new_scid, &token, version, &mut read_buf)
            .map_err(map_quic_error)?;

        read_buf.resize(write_size, 0);

        Ok(QuicListenerHandshake::Response {
            buf: read_buf,
            read_size: buf.len(),
        })
    }

    fn validate_token<'a>(token: &'a [u8], src: &SocketAddr) -> Result<quiche::ConnectionId<'a>> {
        if token.len() < 6 {
            return Err(Error::new(
                ErrorKind::Interrupted,
                format!("Invalid token, token length < 6"),
            ));
        }

        if &token[..6] != b"quiche" {
            return Err(Error::new(
                ErrorKind::Interrupted,
                format!("Invalid token, not start with 'quiche'"),
            ));
        }

        let token = &token[6..];

        let addr = match src.ip() {
            std::net::IpAddr::V4(a) => a.octets().to_vec(),
            std::net::IpAddr::V6(a) => a.octets().to_vec(),
        };

        if token.len() < addr.len() || &token[..addr.len()] != addr.as_slice() {
            return Err(Error::new(
                ErrorKind::Interrupted,
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

#[derive(Clone, PartialEq, Eq, Hash)]
struct QuicListenerAccept;

#[derive(Clone)]
pub struct QuicListener {
    laddrs: Arc<Vec<SocketAddr>>,
    state: Arc<Mutex<QuicListenerState>>,
    event_map: Arc<KeyWaitMap<QuicListenerAccept, ()>>,
    send_map: FuturesWaitMap<QuicConnState, Result<(Vec<u8>, SendInfo)>>,
}

impl QuicListener {
    async fn remove_conn(&self, scid: &ConnectionId<'static>) {
        let mut raw = self.state.lock().await;

        if raw.remove_conn(scid) {
            log::trace!("scid={:?}, remove connection from server pool", scid);
        } else {
            log::warn!(
                "scid={:?}, removed from server pool with error: not found",
                scid
            );
        }
    }
}

impl QuicListener {
    /// Create a new `QuicListener` instance with provided `laddrs` and `config`.
    pub fn new<A: ToSocketAddrs>(laddrs: A, config: Config) -> Result<Self> {
        Ok(QuicListener {
            laddrs: Arc::new(laddrs.to_socket_addrs()?.collect()),
            state: Arc::new(Mutex::new(QuicListenerState::new(config)?)),
            event_map: Arc::new(KeyWaitMap::new()),
            send_map: FuturesWaitMap::new(),
        })
    }

    /// Get the `QuicListener`'s local bound socket address iterator.
    pub fn local_addrs(&self) -> impl Iterator<Item = &SocketAddr> {
        self.laddrs.iter()
    }

    pub async fn send(&self) -> Result<(Vec<u8>, SendInfo)> {
        while let Some((conn, result)) = (&self.send_map).next().await {
            match result {
                Ok((buf, send_info)) => {
                    let send = conn.clone().send_owned();
                    self.send_map.insert(conn, send);

                    return Ok((buf, send_info));
                }
                Err(err) => {
                    log::error!(
                        "QuicConn: id={:?}, send with error, err={}, removed from listener pool",
                        conn.id,
                        err
                    );
                }
            }
        }

        Err(Error::new(ErrorKind::BrokenPipe, "QuicListener broken"))
    }

    /// Processes QUIC packets received from the peer.
    ///
    /// On success the number of bytes processed from the input buffer is returned.
    pub async fn recv<Buf: AsMut<[u8]>>(
        &self,
        mut buf: Buf,
        recv_info: RecvInfo,
    ) -> Result<(usize, Option<Vec<u8>>)> {
        let buf = buf.as_mut();
        let header =
            quiche::Header::from_slice(buf, quiche::MAX_CONN_ID_LEN).map_err(map_quic_error)?;

        let mut state = self.state.lock().await;

        log::trace!("quic listener: {:?}", header);

        if let Some((conn, is_established)) = state.get_conn(&header.dcid) {
            // release the lock before call [QuicConnState::recv] function.
            drop(state);

            let recv_size = match conn.recv(buf, recv_info).await {
                Ok(recv_size) => recv_size,
                Err(err) => {
                    log::error!("conn recv, id={:?}, err={}", conn.id, err);

                    self.remove_conn(&header.dcid).await;

                    return Ok((buf.len(), None));
                }
            };

            if !is_established && conn.is_established().await {
                // relock the state.
                state = self.state.lock().await;
                // move the connection to established set and push state into incoming queue.
                state.established(&header.dcid);

                self.event_map.insert(QuicListenerAccept, ());
            }

            return Ok((recv_size, None));
        }

        // Perform the handshake process.
        match state.handshake(&header, buf, recv_info) {
            Ok(QuicListenerHandshake::Connection {
                conn,
                is_established,
                read_size,
            }) => {
                // notify incoming queue read ops.
                if is_established {
                    self.event_map.insert(QuicListenerAccept, ());
                }

                let send = conn.clone().send_owned();

                self.send_map.insert(conn, send);

                return Ok((read_size, None));
            }
            Ok(QuicListenerHandshake::Response {
                buf,
                read_size: recv_size,
            }) => return Ok((recv_size, Some(buf))),
            Err(err) => {
                log::error!("quic listener handshake, err={}", err);

                return Ok((buf.len(), None));
            }
        }
    }

    /// Accept a new inbound connection.
    pub async fn accept(&self) -> Result<QuicConn> {
        loop {
            let mut state = self.state.lock().await;

            if let Some(conn) = state.incoming_conns.pop_front() {
                return Ok(conn);
            }

            self.event_map.wait(&QuicListenerAccept, state).await;
        }
    }

    /// Returns a stream of incoming connections.
    pub fn incoming(&self) -> impl Stream<Item = Result<QuicConn>> + Send + Unpin {
        Box::pin(unfold(self.clone(), |listener| async {
            let res = listener.accept().await;
            Some((res, listener))
        }))
    }
}
