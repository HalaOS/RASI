use std::collections::{HashMap, VecDeque};
use std::io::{Error, ErrorKind, Result};
use std::sync::Arc;

use futures::lock::Mutex;
use futures::StreamExt;
use futures_waitmap::{FutureWaitMap, WaitMap};
use quiche::{Config, ConnectionId, SendInfo};
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
}

enum QuicListenerEvent {
    Accept(ConnectionId<'static>),
    Send(ConnectionId<'static>),
}

#[derive(Clone)]
pub struct QuicListener {
    state: Arc<Mutex<QuicListenerState>>,
    event_map: Arc<WaitMap<QuicListenerEvent, ()>>,
    send_map: FutureWaitMap<ConnectionId<'static>, Result<(Vec<u8>, SendInfo)>>,
}

impl QuicListener {
    pub async fn send(&self) -> Result<(Vec<u8>, SendInfo)> {
        while let Some((id, result)) = (&self.send_map).next().await {
            match result {
                Ok((buf, send_info)) => {}
                Err(err) => {}
            }
        }

        todo!()
    }
}
