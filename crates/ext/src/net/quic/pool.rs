use std::{
    collections::HashMap,
    fmt::Debug,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use quiche::ConnectionId;
use rand::{seq::SliceRandom, thread_rng};
use rasi::syscall::{global_network, Network};

use crate::utils::AsyncSpinMutex;

use super::{Config, QuicConn, QuicConnector, QuicStream};

enum OpenStream {
    Stream(QuicStream),
    Connector(QuicConnector),
}

struct RawQuicConnPool {
    config: Config,
    conns: HashMap<ConnectionId<'static>, QuicConn>,
}

impl RawQuicConnPool {
    async fn open_stream(
        &mut self,
        server_name: Option<&str>,
        max_conns: usize,
        raddrs: &[SocketAddr],
        syscall: &'static dyn Network,
    ) -> io::Result<OpenStream> {
        let mut conns = self.conns.values().collect::<Vec<_>>();

        conns.shuffle(&mut thread_rng());

        let mut closed = vec![];

        for conn in conns {
            match conn.stream_open(true).await {
                Ok(stream) => {
                    return Ok(OpenStream::Stream(stream));
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(err) => {
                    log::error!("{}, open stream with error: {}", conn, err);
                    closed.push(conn.source_id().clone());
                    continue;
                }
            }
        }

        for id in closed {
            self.conns.remove(&id);
        }

        if !(self.conns.len() < max_conns) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                format!(
                    "Quic conn pool, max connections limits is reached({}).",
                    max_conns
                ),
            ));
        }

        let connector = QuicConnector::new_with(
            server_name,
            ["[::]:0".parse().unwrap(), "0.0.0.0:0".parse().unwrap()].as_slice(),
            raddrs,
            &mut self.config,
            syscall,
        )
        .await?;

        Ok(OpenStream::Connector(connector))
    }
}

/// The quic connection pool implementation.
#[derive(Clone)]
pub struct QuicConnPool {
    server_name: Option<String>,
    /// Peer addresses.
    raddrs: Vec<SocketAddr>,
    /// The maximum number of connections this pool can hold.
    max_conns: usize,
    /// mixin [`RawQuicConnPool`]
    inner: Arc<AsyncSpinMutex<RawQuicConnPool>>,
    /// syscall instance.
    syscall: &'static dyn Network,
}

impl Debug for QuicConnPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "QuicConnPool, max_conns={}, raddrs={:?}",
            self.max_conns, self.raddrs
        )
    }
}

impl QuicConnPool {
    /// Create new [`QuicConn`] pool with custom [`syscall`](Network) interface.
    ///
    /// See [`new`](Self::new) for more information.
    pub fn new_with<A: ToSocketAddrs>(
        server_name: Option<&str>,
        raddrs: A,
        config: Config,
        syscall: &'static dyn Network,
    ) -> io::Result<Self> {
        Ok(Self {
            raddrs: raddrs.to_socket_addrs()?.collect::<Vec<_>>(),
            max_conns: 100,
            syscall,
            server_name: server_name.map(str::to_string),
            inner: Arc::new(AsyncSpinMutex::new(RawQuicConnPool {
                config,
                conns: Default::default(),
            })),
        })
    }
    /// Create new [`QuicConn`] pool with global [`syscall`](Network) interface.
    ///
    pub fn new<A: ToSocketAddrs>(
        server_name: Option<&str>,
        raddrs: A,
        config: Config,
    ) -> io::Result<Self> {
        Self::new_with(server_name, raddrs, config, global_network())
    }

    /// Open new outbound stream.
    ///
    /// This function will randomly select a connection from the pool
    /// and open a new outbound stream.
    ///
    /// If necessary, a new Quic connection will be created.
    /// If the `max_conns` is reached, returns the [`WouldBlock`](io::ErrorKind::WouldBlock) error.
    pub async fn stream_open(&self) -> io::Result<QuicStream> {
        use crate::utils::AsyncLockable;

        let connector = {
            let mut inner = self.inner.lock().await;

            match inner
                .open_stream(
                    self.server_name.as_ref().map(String::as_str),
                    self.max_conns,
                    &self.raddrs,
                    self.syscall,
                )
                .await?
            {
                OpenStream::Stream(stream) => return Ok(stream),
                OpenStream::Connector(connector) => connector,
            }
        };

        // performs real connecting process.

        let connection = connector.connect().await?;

        let stream = connection.stream_open(true).await?;

        // relock inner.
        let mut inner = self.inner.lock().await;

        inner
            .conns
            .insert(connection.source_id().clone(), connection);

        Ok(stream)
    }

    /// Set `max_conns` parameter.
    ///
    /// The default value is `100`
    pub fn set_max_conns(&mut self, value: usize) -> &mut Self {
        self.max_conns = value;
        self
    }
}
