use std::collections::{HashMap, VecDeque};
use std::io::{Error, ErrorKind, Result};
use std::sync::Arc;

use futures::lock::Mutex;
use futures::stream::unfold;
use futures::{Stream, StreamExt};
use futures_waitmap::{FutureWaitMap, WaitMap};
use quiche::{Config, ConnectionId, RecvInfo, SendInfo};
use ring::{hmac::Key, rand::SystemRandom};

use crate::QuicConn;

/// Server-side incoming connection handshake pool.
pub struct QuicListenerState {
    /// The quic config shared between connections for this listener.
    config: Config,
    /// The seed for generating source id
    seed_key: Key,
    /// A quic connection pool that is in the handshake phase
    handshaking_pool: HashMap<ConnectionId<'static>, QuicConn>,
    /// A quic connection pool that is already connected
    established_conns: HashMap<ConnectionId<'static>, QuicConn>,
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

    fn recv<Buf: AsRef<[u8]>>(
        &self,
        buf: Buf,
        recv_info: RecvInfo,
    ) -> Result<(usize, Option<ConnectionId<'static>>)> {
        todo!()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct QuicListenerAccept;

#[derive(Clone)]
pub struct QuicListener {
    state: Arc<Mutex<QuicListenerState>>,
    event_map: Arc<WaitMap<QuicListenerAccept, ()>>,
    send_map: FutureWaitMap<QuicConn, Result<(Vec<u8>, SendInfo)>>,
}

impl QuicListener {
    pub fn new(config: Config) -> Result<Self> {
        Ok(QuicListener {
            state: Arc::new(Mutex::new(QuicListenerState::new(config)?)),
            event_map: Arc::new(WaitMap::new()),
            send_map: FutureWaitMap::new(),
        })
    }
    pub async fn send(&self) -> Result<(Vec<u8>, SendInfo)> {
        while let Some((conn, result)) = (&self.send_map).next().await {
            match result {
                Ok((buf, send_info)) => {
                    let send = conn.clone().into_send();
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
    pub async fn recv<Buf: AsRef<[u8]>>(&self, buf: Buf, recv_info: RecvInfo) -> Result<usize> {
        let state = self.state.lock().await;

        let (recv_size, accept) = state.recv(buf.as_ref(), recv_info)?;

        if let Some(id) = accept {
            log::trace!("new incoming connection, id={:?}", id);
            self.event_map.insert(QuicListenerAccept, ()).await;
        }

        Ok(recv_size)
    }

    /// Accept a new inbound connection.
    pub async fn accept(&self) -> Result<QuicConn> {
        loop {
            let mut state = self.state.lock().await;

            if let Some(conn) = state.incoming_conns.pop_front() {
                return Ok(conn);
            }

            drop(state);

            self.event_map.wait(&QuicListenerAccept).await;
        }
    }

    /// Returns a stream of incoming connections.
    pub fn incoming(&self) -> impl Stream<Item = Result<QuicConn>> + Send {
        unfold(self.clone(), |listener| async {
            let res = listener.accept().await;
            Some((res, listener))
        })
    }
}
