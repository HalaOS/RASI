//! Future-based TCP/IP manipulation operations.

use std::{
    fmt::Debug,
    io::{ErrorKind, Result},
    net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, ToSocketAddrs},
    ops::Deref,
    sync::{Arc, OnceLock},
    task::{Context, Poll},
};

use futures::{future::poll_fn, AsyncRead, AsyncWrite, Stream};

/// A network driver must implement the `Driver-*` traits in this module.
pub mod syscall {
    use super::*;

    #[cfg(unix)]
    pub mod unix {
        use super::*;
        pub trait DriverUnixListener: Sync + Send {
            /// Returns the local socket address of this listener.
            fn local_addr(&self) -> Result<std::os::unix::net::SocketAddr>;

            /// Polls and accepts a new incoming connection to this listener.
            ///
            /// When a connection is established, the corresponding stream and address will be returned.
            fn poll_next(
                &self,
                cx: &mut Context<'_>,
            ) -> Poll<Result<(crate::net::unix::UnixStream, std::os::unix::net::SocketAddr)>>;
        }

        pub trait DriverUnixStream: Sync + Send {
            /// Returns the local address that this stream is connected from.
            fn local_addr(&self) -> Result<std::os::unix::net::SocketAddr>;

            /// Returns the local address that this stream is connected to.
            fn peer_addr(&self) -> Result<std::os::unix::net::SocketAddr>;

            /// Shuts down the read, write, or both halves of this connection.
            ///
            /// This method will cause all pending and future I/O on the specified portions to return
            /// immediately with an appropriate value (see the documentation of [`Shutdown`]).
            ///
            /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
            fn shutdown(&self, how: Shutdown) -> Result<()>;

            /// poll and receives data from the socket.
            ///
            /// On success, returns the number of bytes read.
            fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>>;

            /// Sends data on the socket to the remote address
            ///
            /// On success, returns the number of bytes written.
            fn poll_write(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;

            /// Poll and wait underlying tcp connection established event.
            ///
            /// This function is no effect for server side socket created
            /// by [`poll_next`](DriverUnixListener::poll_next) function.
            fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;
        }
    }

    /// A driver is the main entry to access asynchronously network functions.
    pub trait Driver: Send + Sync {
        /// Create a new tcp listener socket with provided `laddrs`.
        fn tcp_listen(&self, laddrs: &[SocketAddr]) -> Result<TcpListener>;

        #[cfg(unix)]
        unsafe fn tcp_listener_from_raw_fd(&self, fd: std::os::fd::RawFd) -> Result<TcpListener>;

        #[cfg(windows)]
        unsafe fn tcp_listener_from_raw_socket(
            &self,
            socket: std::os::windows::io::RawSocket,
        ) -> Result<TcpListener>;

        /// Create a new `TcpStream` and connect to `raddrs`.
        ///  
        /// When this function returns a [`TcpStream`] object the underlying
        /// tcp connection may not be actually established, and the user
        /// needs to manually call the poll_ready method to wait for the
        /// connection to be established.
        fn tcp_connect(&self, raddrs: &SocketAddr) -> Result<TcpStream>;

        #[cfg(unix)]
        unsafe fn tcp_stream_from_raw_fd(&self, fd: std::os::fd::RawFd) -> Result<TcpStream>;

        #[cfg(windows)]
        unsafe fn tcp_stream_from_raw_socket(
            &self,
            socket: std::os::windows::io::RawSocket,
        ) -> Result<TcpStream>;

        /// Create new `UdpSocket` which will be bound to the specified `laddrs`
        fn udp_bind(&self, laddrs: &[SocketAddr]) -> Result<UdpSocket>;

        #[cfg(unix)]
        unsafe fn udp_from_raw_fd(&self, fd: std::os::fd::RawFd) -> Result<UdpSocket>;

        #[cfg(windows)]
        unsafe fn udp_from_raw_socket(
            &self,
            socket: std::os::windows::io::RawSocket,
        ) -> Result<UdpSocket>;

        #[cfg(unix)]
        #[cfg_attr(docsrs, doc(cfg(unix)))]
        fn unix_listen(&self, path: &std::path::Path) -> Result<crate::net::unix::UnixListener>;

        #[cfg(unix)]
        #[cfg_attr(docsrs, doc(cfg(unix)))]
        fn unix_connect(&self, path: &std::path::Path) -> Result<crate::net::unix::UnixStream>;
    }

    /// Driver-specific `TcpListener` implementation must implement this trait.
    ///
    /// When this trait object is dropping, the implementition must close the internal tcp listener socket.
    pub trait DriverTcpListener: Sync + Send {
        /// Returns the local socket address of this listener.
        fn local_addr(&self) -> Result<SocketAddr>;

        /// Gets the value of the IP_TTL option for this socket.
        /// For more information about this option, see [`set_ttl`](DriverTcpListener::set_ttl).
        fn ttl(&self) -> Result<u32>;

        /// Sets the value for the IP_TTL option on this socket.
        /// This value sets the time-to-live field that is used in every packet sent from this socket.
        fn set_ttl(&self, ttl: u32) -> Result<()>;

        /// Polls and accepts a new incoming connection to this listener.
        ///
        /// When a connection is established, the corresponding stream and address will be returned.
        fn poll_next(&self, cx: &mut Context<'_>) -> Poll<Result<(TcpStream, SocketAddr)>>;
    }

    /// Driver-specific `TcpStream` implementation must implement this trait.
    ///
    /// When this trait object is dropping, the implementition must close the internal tcp listener socket.
    pub trait DriverTcpStream: Sync + Send + Debug {
        /// Returns the local address that this stream is connected from.
        fn local_addr(&self) -> Result<SocketAddr>;

        /// Returns the local address that this stream is connected to.
        fn peer_addr(&self) -> Result<SocketAddr>;

        /// Gets the value of the IP_TTL option for this socket.
        /// For more information about this option, see [`set_ttl`](DriverTcpStream::set_ttl).
        fn ttl(&self) -> Result<u32>;

        /// Sets the value for the IP_TTL option on this socket.
        /// This value sets the time-to-live field that is used in every packet sent from this socket.
        fn set_ttl(&self, ttl: u32) -> Result<()>;

        /// Gets the value of the `TCP_NODELAY` option on this socket.
        ///
        /// For more information about this option, see [`set_nodelay`](DriverTcpStream::set_nodelay).
        fn nodelay(&self) -> Result<bool>;

        /// Sets the value of the `TCP_NODELAY` option on this socket.
        ///
        /// If set, this option disables the Nagle algorithm. This means that
        /// segments are always sent as soon as possible, even if there is only a
        /// small amount of data. When not set, data is buffered until there is a
        /// sufficient amount to send out, thereby avoiding the frequent sending of
        /// small packets.
        fn set_nodelay(&self, nodelay: bool) -> Result<()>;

        /// Shuts down the read, write, or both halves of this connection.
        ///
        /// This method will cause all pending and future I/O on the specified portions to return
        /// immediately with an appropriate value (see the documentation of [`Shutdown`]).
        ///
        /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
        fn shutdown(&self, how: Shutdown) -> Result<()>;

        /// poll and receives data from the socket.
        ///
        /// On success, returns the number of bytes read.
        fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>>;

        /// Sends data on the socket to the remote address
        ///
        /// On success, returns the number of bytes written.
        fn poll_write(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;

        /// Poll and wait underlying tcp connection established event.
        ///
        /// This function is no effect for server side socket created
        /// by [`poll_next`](DriverTcpListener::poll_next) function.
        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;
    }

    /// Driver-specific `UdpSocket` implementation
    ///
    /// When this trait object is dropping, the implementition must close the internal tcp socket.
    pub trait DriverUdpSocket: Sync + Send {
        /// Shuts down the read, write, or both halves of this connection.
        ///
        /// This method will cause all pending and future I/O on the specified portions to return
        /// immediately with an appropriate value (see the documentation of [`Shutdown`]).
        ///
        /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
        fn shutdown(&self, how: Shutdown) -> Result<()>;

        /// Returns the local address that this stream is connected from.
        fn local_addr(&self) -> Result<SocketAddr>;

        /// Returns the local address that this stream is connected to.
        fn peer_addr(&self) -> Result<SocketAddr>;

        /// Gets the value of the IP_TTL option for this socket.
        /// For more information about this option, see [`set_ttl`](DriverUdpSocket::set_ttl).
        fn ttl(&self) -> Result<u32>;

        /// Sets the value for the IP_TTL option on this socket.
        /// This value sets the time-to-live field that is used in every packet sent from this socket.
        fn set_ttl(&self, ttl: u32) -> Result<()>;

        /// Executes an operation of the IP_ADD_MEMBERSHIP type.
        ///
        /// This function specifies a new multicast group for this socket to join.
        /// The address must be a valid multicast address, and interface is the
        /// address of the local interface with which the system should join the
        /// multicast group. If it’s equal to INADDR_ANY then an appropriate
        /// interface is chosen by the system.
        fn join_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> Result<()>;

        /// Executes an operation of the `IPV6_ADD_MEMBERSHIP` type.
        ///
        /// This function specifies a new multicast group for this socket to join.
        /// The address must be a valid multicast address, and `interface` is the
        /// index of the interface to join/leave (or 0 to indicate any interface).
        fn join_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> Result<()>;

        /// Executes an operation of the `IP_DROP_MEMBERSHIP` type.
        ///
        /// For more information about this option, see
        /// [`join_multicast_v4`][link].
        ///
        /// [link]: #method.join_multicast_v4
        fn leave_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> Result<()>;

        /// Executes an operation of the `IPV6_DROP_MEMBERSHIP` type.
        ///
        /// For more information about this option, see
        /// [`join_multicast_v6`][link].
        ///
        /// [link]: #method.join_multicast_v6
        fn leave_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> Result<()>;

        /// Sets the value of the IP_MULTICAST_LOOP option for this socket.
        ///
        /// If enabled, multicast packets will be looped back to the local socket. Note that this might not have any effect on IPv6 sockets.
        fn set_multicast_loop_v4(&self, on: bool) -> Result<()>;

        /// Sets the value of the IPV6_MULTICAST_LOOP option for this socket.
        ///
        /// Controls whether this socket sees the multicast packets it sends itself. Note that this might not have any affect on IPv4 sockets.
        fn set_multicast_loop_v6(&self, on: bool) -> Result<()>;

        /// Gets the value of the IP_MULTICAST_LOOP option for this socket.
        fn multicast_loop_v4(&self) -> Result<bool>;

        /// Gets the value of the IPV6_MULTICAST_LOOP option for this socket.
        fn multicast_loop_v6(&self) -> Result<bool>;

        /// Sets the value of the SO_BROADCAST option for this socket.
        /// When enabled, this socket is allowed to send packets to a broadcast address.
        fn set_broadcast(&self, on: bool) -> Result<()>;

        /// Gets the value of the SO_BROADCAST option for this socket.
        /// For more information about this option, see [`set_broadcast`](DriverUdpSocket::set_broadcast).
        fn broadcast(&self) -> Result<bool>;

        /// Receives data from the socket.
        ///
        /// On success, returns the number of bytes read and the origin.
        fn poll_recv_from(
            &self,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<(usize, SocketAddr)>>;

        /// Sends data on the socket to the given `target` address.
        ///
        /// On success, returns the number of bytes written.
        fn poll_send_to(
            &self,
            cx: &mut Context<'_>,
            buf: &[u8],
            peer: SocketAddr,
        ) -> Poll<Result<usize>>;
    }
}

/// A TCP socket server, listening for connections.
/// After creating a TcpListener by binding it to a socket address,
/// it listens for incoming TCP connections. These can be accepted
/// by awaiting elements from the async stream of incoming connections.
///
/// The socket will be closed when the value is dropped.
/// The Transmission Control Protocol is specified in IETF RFC 793.
/// This type is an async version of std::net::TcpListener.
pub struct TcpListener(Box<dyn syscall::DriverTcpListener>);

impl<T: syscall::DriverTcpListener + 'static> From<T> for TcpListener {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Deref for TcpListener {
    type Target = dyn syscall::DriverTcpListener;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl TcpListener {
    /// Returns inner [`syscall::DriverTcpListener`] ptr.
    pub fn as_raw_ptr(&self) -> &dyn syscall::DriverTcpListener {
        &*self.0
    }

    /// See [`poll_next`](syscall::DriverTcpListener::poll_next) for more information.
    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        poll_fn(|cx| self.poll_next(cx)).await
    }

    /// Create new `TcpListener` which will be bound to the specified `laddrs`
    pub async fn bind<L: ToSocketAddrs>(laddrs: L) -> Result<Self> {
        Self::bind_with(laddrs, get_network_driver()).await
    }

    /// Use custom `NetworkDriver` to create new `TcpListener` which will be bound to the specified `laddrs`.
    pub async fn bind_with<L: ToSocketAddrs>(
        laddrs: L,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        let laddrs = laddrs.to_socket_addrs()?.collect::<Vec<_>>();
        driver.tcp_listen(&laddrs)
    }

    #[cfg(unix)]
    pub unsafe fn from_raw_fd_with(
        fd: std::os::fd::RawFd,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        driver.tcp_listener_from_raw_fd(fd)
    }

    #[cfg(unix)]
    pub unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Result<Self> {
        Self::from_raw_fd_with(fd, get_network_driver())
    }

    #[cfg(windows)]
    pub unsafe fn from_raw_socket_with(
        fd: std::os::windows::io::RawSocket,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        driver.tcp_listener_from_raw_socket(fd)
    }

    #[cfg(windows)]
    pub unsafe fn from_raw_socket(fd: std::os::windows::io::RawSocket) -> Result<Self> {
        Self::from_raw_socket_with(fd, get_network_driver())
    }
}

impl Stream for TcpListener {
    type Item = Result<TcpStream>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.as_raw_ptr().poll_next(cx) {
            Poll::Ready(Ok((stream, _))) => Poll::Ready(Some(Ok(stream))),
            Poll::Ready(Err(err)) => {
                if err.kind() == ErrorKind::BrokenPipe {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Err(err)))
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A TCP stream between a local and a remote socket.
///
/// A `TcpStream` can either be created by connecting to an endpoint, via the [`connect`] method,
/// or by [accepting] a connection from a [listener].  It can be read or written to using the
/// [`AsyncRead`], [`AsyncWrite`], and related extension traits in [`futures::io`].
///
/// The connection will be closed when the value is dropped. The reading and writing portions of
/// the connection can also be shut down individually with the [`shutdown`] method.
///
/// This type is an async version of [`std::net::TcpStream`].
///
/// [`connect`]: struct.TcpStream.html#method.connect
/// [accepting]: struct.TcpListener.html#method.accept
/// [listener]: struct.TcpListener.html
/// [`AsyncRead`]: https://docs.rs/futures/0.3/futures/io/trait.AsyncRead.html
/// [`AsyncWrite`]: https://docs.rs/futures/0.3/futures/io/trait.AsyncWrite.html
/// [`futures::io`]: https://docs.rs/futures/0.3/futures/io/index.html
/// [`shutdown`]: struct.TcpStream.html#method.shutdown
/// [`std::net::TcpStream`]: https://doc.rust-lang.org/std/net/struct.TcpStream.html
#[derive(Debug, Clone)]
pub struct TcpStream(Arc<Box<dyn syscall::DriverTcpStream>>);

impl<T: syscall::DriverTcpStream + 'static> From<T> for TcpStream {
    fn from(value: T) -> Self {
        Self(Arc::new(Box::new(value)))
    }
}

impl Deref for TcpStream {
    type Target = dyn syscall::DriverTcpStream;
    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}

impl TcpStream {
    /// Returns inner [`syscall::DriverTcpStream`] ptr.
    pub fn as_raw_ptr(&self) -> &dyn syscall::DriverTcpStream {
        &**self.0
    }

    /// Connect to peer by provided `raddrs`.
    pub async fn connect<R: ToSocketAddrs>(raddrs: R) -> Result<Self> {
        Self::connect_with(raddrs, get_network_driver()).await
    }

    /// Use custom `NetworkDriver` to connect to peer by provided `raddrs`.
    pub async fn connect_with<R: ToSocketAddrs>(
        raddrs: R,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        let mut last_error = None;

        for raddr in raddrs.to_socket_addrs()?.collect::<Vec<_>>() {
            match driver.tcp_connect(&raddr) {
                Ok(stream) => {
                    // Wait for the asynchronously connecting to complete
                    match poll_fn(|cx| stream.poll_ready(cx)).await {
                        Ok(()) => {
                            return Ok(stream);
                        }
                        Err(err) => {
                            last_error = Some(err);
                        }
                    }
                }
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap())
    }

    #[cfg(unix)]
    pub unsafe fn from_raw_fd_with(
        fd: std::os::fd::RawFd,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        driver.tcp_stream_from_raw_fd(fd)
    }

    #[cfg(unix)]
    pub unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Result<Self> {
        Self::from_raw_fd_with(fd, get_network_driver())
    }

    #[cfg(windows)]
    pub unsafe fn from_raw_socket_with(
        fd: std::os::windows::io::RawSocket,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        driver.tcp_stream_from_raw_socket(fd)
    }

    #[cfg(windows)]
    pub unsafe fn from_raw_socket(fd: std::os::windows::io::RawSocket) -> Result<Self> {
        Self::from_raw_socket_with(fd, get_network_driver())
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.as_raw_ptr().poll_read(cx, buf)
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.as_raw_ptr().poll_write(cx, buf)
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.shutdown(Shutdown::Both)?;

        Poll::Ready(Ok(()))
    }
}

/// A UDP socket.
///
/// After creating a `UdpSocket` by [`bind`]ing it to a socket address, data can be [sent to] and
/// [received from] any other socket address.
///
/// As stated in the User Datagram Protocol's specification in [IETF RFC 768], UDP is an unordered,
/// unreliable protocol. Refer to [`TcpListener`] and [`TcpStream`] for async TCP primitives.
///
/// This type is an async version of [`std::net::UdpSocket`].
///
/// [`bind`]: #method.bind
/// [received from]: #method.recv_from
/// [sent to]: #method.send_to
/// [`TcpListener`]: struct.TcpListener.html
/// [`TcpStream`]: struct.TcpStream.html
/// [`std::net`]: https://doc.rust-lang.org/std/net/index.html
/// [IETF RFC 768]: https://tools.ietf.org/html/rfc768
/// [`std::net::UdpSocket`]: https://doc.rust-lang.org/std/net/struct.UdpSocket.html
///
#[derive(Clone)]
pub struct UdpSocket(Arc<Box<dyn syscall::DriverUdpSocket>>);

impl<T: syscall::DriverUdpSocket + 'static> From<T> for UdpSocket {
    fn from(value: T) -> Self {
        Self(Arc::new(Box::new(value)))
    }
}

impl Deref for UdpSocket {
    type Target = dyn syscall::DriverUdpSocket;
    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}

impl UdpSocket {
    /// Returns inner driver-specific implementation.
    pub fn as_raw_ptr(&self) -> &dyn syscall::DriverUdpSocket {
        &**self.0
    }

    /// Create new udp socket which will be bound to the specified `laddrs`
    pub async fn bind<L: ToSocketAddrs>(laddrs: L) -> Result<Self> {
        Self::bind_with(laddrs, get_network_driver()).await
    }

    pub async fn bind_with<L: ToSocketAddrs>(
        laddrs: L,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        let laddrs = laddrs.to_socket_addrs()?.collect::<Vec<_>>();
        driver.udp_bind(&laddrs)
    }

    #[cfg(unix)]
    pub unsafe fn from_raw_fd_with(
        fd: std::os::fd::RawFd,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        driver.udp_from_raw_fd(fd)
    }

    #[cfg(unix)]
    pub unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Result<Self> {
        Self::from_raw_fd_with(fd, get_network_driver())
    }

    #[cfg(windows)]
    pub unsafe fn from_raw_socket_with(
        fd: std::os::windows::io::RawSocket,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        driver.udp_from_raw_socket(fd)
    }

    #[cfg(windows)]
    pub unsafe fn from_raw_socket(fd: std::os::windows::io::RawSocket) -> Result<Self> {
        Self::from_raw_socket_with(fd, get_network_driver())
    }

    /// Sends data on the socket to the given `target` address.
    ///
    /// On success, returns the number of bytes written.
    pub async fn send_to<Buf: AsRef<[u8]>, A: ToSocketAddrs>(
        &self,
        buf: Buf,
        target: A,
    ) -> Result<usize> {
        let mut last_error = None;

        let buf = buf.as_ref();

        for raddr in target.to_socket_addrs()? {
            match poll_fn(|cx| self.poll_send_to(cx, buf, raddr)).await {
                Ok(send_size) => return Ok(send_size),
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the origin.
    pub async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        poll_fn(|cx| self.poll_recv_from(cx, buf)).await
    }
}

/// Unix-specific sockets implementation.
#[cfg(unix)]
#[cfg_attr(docsrs, doc(cfg(unix)))]
pub mod unix {

    use super::*;
    use std::path::Path;

    use super::syscall::unix::*;

    /// A unix domain server-side socket.
    pub struct UnixListener(Box<dyn DriverUnixListener>);

    impl<T: DriverUnixListener + 'static> From<T> for UnixListener {
        fn from(value: T) -> Self {
            Self(Box::new(value))
        }
    }

    impl Deref for UnixListener {
        type Target = dyn DriverUnixListener;
        fn deref(&self) -> &Self::Target {
            &*self.0
        }
    }

    impl UnixListener {
        /// Returns inner driver-specific implementation.
        pub fn as_raw_ptr(&self) -> &dyn DriverUnixListener {
            &*self.0
        }

        /// See [`poll_next`](syscall::unix::DriverUnixListener::poll_next) for more information.
        pub async fn accept(&self) -> Result<(UnixStream, std::os::unix::net::SocketAddr)> {
            poll_fn(|cx| self.poll_next(cx)).await
        }

        /// Create new `TcpListener` which will be bound to the specified `laddrs`
        pub async fn bind<P: AsRef<Path>>(path: P) -> Result<Self> {
            Self::bind_with(path, get_network_driver()).await
        }

        /// Use custom `NetworkDriver` to create new `UnixListener` which will be bound to the specified `laddrs`.
        pub async fn bind_with<P: AsRef<Path>>(
            path: P,
            driver: &dyn syscall::Driver,
        ) -> Result<Self> {
            driver.unix_listen(path.as_ref())
        }
    }

    impl Stream for UnixListener {
        type Item = Result<UnixStream>;

        fn poll_next(
            self: std::pin::Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>> {
            match self.as_raw_ptr().poll_next(cx) {
                Poll::Ready(Ok((stream, _))) => Poll::Ready(Some(Ok(stream))),
                Poll::Ready(Err(err)) => {
                    if err.kind() == ErrorKind::BrokenPipe {
                        Poll::Ready(None)
                    } else {
                        Poll::Ready(Some(Err(err)))
                    }
                }
                Poll::Pending => Poll::Pending,
            }
        }
    }

    /// A unix domain stream between a local and a remote socket.
    #[derive(Clone)]
    pub struct UnixStream(Arc<Box<dyn DriverUnixStream>>);

    impl<T: DriverUnixStream + 'static> From<T> for UnixStream {
        fn from(value: T) -> Self {
            Self(Arc::new(Box::new(value)))
        }
    }

    impl Deref for UnixStream {
        type Target = dyn DriverUnixStream;
        fn deref(&self) -> &Self::Target {
            &**self.0
        }
    }

    impl UnixStream {
        /// Returns inner driver-specific implementation.
        pub fn as_raw_ptr(&self) -> &dyn DriverUnixStream {
            &**self.0
        }

        /// Connect to peer by provided `raddrs`.
        pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
            Self::connect_with(path, get_network_driver()).await
        }

        /// Use custom `NetworkDriver` to connect to peer by provided `raddrs`.
        pub async fn connect_with<P: AsRef<Path>>(
            path: P,
            driver: &dyn syscall::Driver,
        ) -> Result<Self> {
            let stream = driver.unix_connect(path.as_ref())?;

            // Wait for the asynchronously connecting to complete
            poll_fn(|cx| stream.poll_ready(cx)).await?;

            Ok(stream)
        }
    }

    impl AsyncRead for UnixStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize>> {
            self.as_raw_ptr().poll_read(cx, buf)
        }
    }

    impl AsyncWrite for UnixStream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            self.as_raw_ptr().poll_write(cx, buf)
        }

        fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            self.shutdown(Shutdown::Both)?;

            Poll::Ready(Ok(()))
        }
    }
}

static NETWORK_DRIVER: OnceLock<Box<dyn syscall::Driver>> = OnceLock::new();

/// Get global register `NetworkDriver` instance.
pub fn get_network_driver() -> &'static dyn syscall::Driver {
    NETWORK_DRIVER
        .get()
        .expect("Call register_network_driver first.")
        .as_ref()
}

/// Register provided [`syscall::Driver`] as global network implementation.
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_network_driver<E: syscall::Driver + 'static>(driver: E) {
    if NETWORK_DRIVER.set(Box::new(driver)).is_err() {
        panic!("Multiple calls to register_global_network are not permitted!!!");
    }
}
