use std::{fmt::Debug, sync::Arc};

use futures::lock::Mutex;
use futures_waitmap::WaitMap;
use quiche::ConnectionId;

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

struct QuicConnState {
    conn: quiche::Connection,
}

impl Debug for QuicConnState {
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

impl QuicConnState {
    fn new(conn: quiche::Connection) -> Self {
        QuicConnState { conn }
    }
}

/// A Quic connection between a local and a remote socket.
#[derive(Clone)]
pub struct QuicConn {
    state: Arc<Mutex<QuicConnState>>,
    event_map: Arc<WaitMap<QuicConnStateEvent, ()>>,
}

impl From<quiche::Connection> for QuicConn {
    fn from(value: quiche::Connection) -> Self {
        Self {
            state: Arc::new(Mutex::new(QuicConnState::new(value))),
            event_map: Arc::new(WaitMap::new()),
        }
    }
}
