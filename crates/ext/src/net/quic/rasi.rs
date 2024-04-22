//! This mod is the rasi compatibility layer of quic protocol.

use std::{
    fmt::{Debug, Display},
    io,
    net::{SocketAddr, ToSocketAddrs},
    ops,
    sync::Arc,
};

use quiche::{ConnectionId, RecvInfo};
use rand::{seq::IteratorRandom, thread_rng};
use rasi::{
    executor::spawn,
    future::FutureExt,
    io::{AsyncRead, AsyncWrite},
    stream::TryStreamExt,
    syscall::{global_network, Network},
    time::TimeoutExt,
};

use crate::{
    net::udp_group::{self, PathInfo, UdpGroup},
    utils::ReadBuf,
};

use super::{
    state::{QuicConnState, QuicListenerState, QuicServerStateIncoming},
    Config,
};

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
    pub udp_group: UdpGroup,
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
/// A `QuicConn` can either be created by connecting to an endpoint, via the [`QuicConnector`],
/// or by [accepting](super::QuicListener::accept) a connection from a [`listener`](super::QuicListener).
///
/// You can either open a stream via the [`open_stream`](Self::stream_open) function,
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
    pub async fn close(&self) -> io::Result<()> {
        self.inner.0.close(false, 0, b"").await
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

    pub(crate) async fn send_loop_inner(
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

/// The server socket for quic to listen and accept newly incoming connections.
///
/// This implementation use [`UdpGroup`] to listen on a range local ports.
/// unlike the TcpListener, the QuicListener will listen on every address passed in via [`ToSocketAddrs`].
///
/// The socket will be closed when the value is dropped.
pub struct QuicListener {
    incoming: QuicServerStateIncoming,
    laddrs: Vec<SocketAddr>,
}

impl QuicListener {
    /// Creates a new `QuicListener` with custom [syscall](rasi::syscall::Network) which will be bound to the specified address.
    /// The returned listener is ready for accepting connections.
    /// Binding with a port number of 0 will request that the OS assigns a port to this listener.
    /// The port allocated can be queried via the local_addr method.
    ///
    /// See [`bind`](QuicListener::bind) for more information.
    pub async fn bind_with<A: ToSocketAddrs>(
        laddrs: A,
        config: Config,
        syscall: &'static dyn Network,
    ) -> io::Result<Self> {
        let max_send_udp_payload_size = config.max_send_udp_payload_size;

        let socket = UdpGroup::bind_with(laddrs, syscall).await?;

        let laddrs = socket.local_addrs().cloned().collect::<Vec<_>>();

        // bin udp group.
        let (sender, receiver) = socket.split();

        // create inner sm.
        let (state, incoming) = QuicListenerState::new(config)?;

        // start udp recv loop
        rasi::executor::spawn(Self::recv_loop(
            state,
            receiver,
            sender,
            max_send_udp_payload_size,
        ));

        Ok(Self { incoming, laddrs })
    }

    /// Creates a new `QuicListener` with global registered [syscall](rasi::syscall::Network)
    /// which will be bound to the specified address.
    /// The returned listener is ready for accepting connections.
    /// Binding with a port number of 0 will request that the OS assigns a port to this listener.
    /// The port allocated can be queried via the local_addr method.
    ///
    /// See [`bind`](Self::bind) for more information.
    pub async fn bind<A: ToSocketAddrs>(laddrs: A, config: Config) -> io::Result<Self> {
        Self::bind_with(laddrs, config, global_network()).await
    }

    /// Accepts a new incoming stream via this quic connection.
    pub async fn accept(&self) -> Option<QuicConn> {
        self.incoming
            .accept()
            .await
            .map(|state| QuicConn::new(state))
    }

    /// Returns the local addresses iterator that this quic listener is bound to.
    pub fn local_addrs(&self) -> impl Iterator<Item = &SocketAddr> {
        self.laddrs.iter()
    }
}

impl QuicListener {
    async fn recv_loop(
        state: QuicListenerState,
        receiver: udp_group::Receiver,
        sender: udp_group::Sender,
        max_send_udp_payload_size: usize,
    ) {
        match Self::recv_loop_inner(state, receiver, sender, max_send_udp_payload_size).await {
            Ok(_) => {
                log::info!("QuicListener recv_loop stopped.");
            }
            Err(err) => {
                log::error!("QuicListener recv_loop stopped with err: {}", err);
            }
        }
    }

    async fn recv_loop_inner(
        state: QuicListenerState,
        mut receiver: udp_group::Receiver,
        sender: udp_group::Sender,
        max_send_udp_payload_size: usize,
    ) -> io::Result<()> {
        while let Some((mut buf, path_info)) = receiver.try_next().await? {
            let (_, resp, conn_state) = match state
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
                    log::error!("handle packet from {:?} error: {}", path_info, err);
                    continue;
                }
            };

            if let Some(resp) = resp {
                sender.send_to_on_path(&resp, path_info.reverse()).await?;
            }

            if let Some(conn_state) = conn_state {
                spawn(Self::send_loop(
                    state.clone(),
                    conn_state,
                    sender.clone(),
                    max_send_udp_payload_size,
                ));
            }
        }

        Ok(())
    }

    async fn send_loop(
        state: QuicListenerState,
        conn_state: QuicConnState,
        sender: udp_group::Sender,
        max_send_udp_payload_size: usize,
    ) {
        match QuicConn::send_loop_inner(conn_state.clone(), sender, max_send_udp_payload_size).await
        {
            Ok(_) => {
                log::trace!("{}, stop send loop", conn_state);
            }
            Err(err) => {
                log::error!("{}, stop send loop with error: {}", conn_state, err);
            }
        }

        state.remove_conn(&conn_state.scid).await;
    }
}
