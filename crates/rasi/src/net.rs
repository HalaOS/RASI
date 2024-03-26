//! Networking primitives for TCP/UDP communication.
//!
//! This module provides networking functionality for the Transmission Control and User Datagram Protocols, as well as types for IP and socket addresses.
//!
//! This module is an async version of std::net.
//!
use std::{
    fmt::{Debug, Display},
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    sync::Arc,
    task::Poll,
};

use futures::{AsyncRead, AsyncWrite};
use rasi_syscall::{global_network, Handle, Network};

use crate::utils::cancelable_would_block;

/// A TCP socket server, listening for connections.
/// After creating a TcpListener by binding it to a socket address,
/// it listens for incoming TCP connections. These can be accepted
/// by awaiting elements from the async stream of incoming connections.
///
/// The socket will be closed when the value is dropped.
/// The Transmission Control Protocol is specified in IETF RFC 793.
/// This type is an async version of std::net::TcpListener.
pub struct TcpListener {
    /// syscall socket handle.
    sys_socket: rasi_syscall::Handle,

    /// a reference to syscall interface .
    syscall: &'static dyn Network,
}

impl TcpListener {
    /// Creates a new TcpListener with custom [syscall](rasi_syscall::Network) which will be bound to the specified address.
    /// The returned listener is ready for accepting connections.
    /// Binding with a port number of 0 will request that the OS assigns a port to this listener.
    /// The port allocated can be queried via the local_addr method.
    ///
    /// See [`bind`](TcpListener::bind) for more information.
    pub async fn bind_with<A: ToSocketAddrs>(
        laddrs: A,
        syscall: &'static dyn Network,
    ) -> io::Result<Self> {
        let laddrs = laddrs.to_socket_addrs()?.collect::<Vec<_>>();

        let sys_socket =
            cancelable_would_block(|cx| syscall.tcp_listener_bind(cx.waker().clone(), &laddrs))
                .await?;

        Ok(TcpListener {
            sys_socket,
            syscall,
        })
    }

    /// Creates a new TcpListener with global [syscall](rasi_syscall::Network) which will be bound to the specified address.
    /// The returned listener is ready for accepting connections.
    /// Binding with a port number of 0 will request that the OS assigns a port to this listener.
    /// The port allocated can be queried via the local_addr method.
    ///
    /// Use function [`bind`](TcpListener::bind_with) to specified custom [syscall](rasi_syscall::Network)
    ///
    /// # Examples
    /// Create a TCP listener bound to 127.0.0.1:0:
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn bind<A: ToSocketAddrs>(laddrs: A) -> io::Result<Self> {
        Self::bind_with(laddrs, global_network()).await
    }

    /// Accepts a new incoming connection to this listener.
    ///
    /// When a connection is established, the corresponding stream and address will be returned.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// let (stream, addr) = listener.accept().await?;
    /// #
    /// # Ok(()) }) }
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (sys_socket, raddr) = cancelable_would_block(|cx| {
            self.syscall
                .tcp_listener_accept(cx.waker().clone(), &self.sys_socket)
        })
        .await?;

        Ok((TcpStream::new(sys_socket, self.syscall), raddr))
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, to identify when binding to port 0 which port was assigned
    /// by the OS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:8080").await?;
    /// let addr = listener.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.syscall.tcp_listener_local_addr(&self.sys_socket)
    }

    /// Gets the value of the IP_TTL option for this socket.
    /// For more information about this option, see [`set_ttl`](Self::set_ttl).
    pub fn ttl(&self) -> io::Result<u32> {
        self.syscall.tcp_listener_ttl(&self.sys_socket)
    }

    /// Sets the value for the IP_TTL option on this socket.
    /// This value sets the time-to-live field that is used in every packet sent from this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.syscall.tcp_listener_set_ttl(&self.sys_socket, ttl)
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
///
/// ## Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
/// #
/// use rasi::net::TcpStream;
/// use rasi::prelude::*;
///
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
/// stream.write_all(b"hello world").await?;
///
/// let mut buf = vec![0u8; 1024];
/// let n = stream.read(&mut buf).await?;
/// #
/// # Ok(()) }) }
/// ```
pub struct TcpStream {
    /// syscall socket handle.
    sys_socket: rasi_syscall::Handle,

    /// a reference to syscall interface .
    syscall: &'static dyn Network,

    /// The cancel handle reference to latest pending write ops.
    write_cancel_handle: Option<Handle>,

    /// The cancel handle reference to latest pending read ops.
    read_cancel_handle: Option<Handle>,
}

impl Debug for TcpStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TcpStream {:?} => {:?}",
            self.local_addr(),
            self.peer_addr()
        )
    }
}

impl TcpStream {
    fn new(sys_socket: Handle, syscall: &'static dyn Network) -> Self {
        Self {
            sys_socket,
            syscall,
            write_cancel_handle: None,
            read_cancel_handle: None,
        }
    }
    /// Using custom [`syscall`](rasi_syscall::Network) interface to create a new TCP
    /// stream connected to the specified address.
    ///
    /// see [`connect`](TcpStream::connect) for more information.
    pub async fn connect_with<A: ToSocketAddrs>(
        raddrs: A,
        syscall: &'static dyn Network,
    ) -> io::Result<Self> {
        let raddrs = raddrs.to_socket_addrs()?.collect::<Vec<_>>();

        let sys_socket =
            cancelable_would_block(|cx| syscall.tcp_stream_connect(cx.waker().clone(), &raddrs))
                .await?;

        Ok(Self::new(sys_socket, syscall))
    }

    /// Using global [`syscall`](rasi_syscall::Network) interface to create a new TCP
    /// stream connected to the specified address.
    ///
    /// This method will create a new TCP socket and attempt to connect it to the `addr`
    /// provided. The [returned future] will be resolved once the stream has successfully
    /// connected, or it will return an error if one occurs.
    ///
    /// [returned future]: struct.Connect.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn connect<A: ToSocketAddrs>(raddrs: A) -> io::Result<Self> {
        Self::connect_with(raddrs, global_network()).await
    }

    /// Returns the local address that this stream is connected to.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let addr = stream.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.syscall.tcp_stream_local_addr(&self.sys_socket)
    }

    /// Returns the remote address that this stream is connected to.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let peer = stream.peer_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.syscall.tcp_stream_remote_addr(&self.sys_socket)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`].
    ///
    /// [`set_ttl`]: #method.set_ttl
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_ttl(100)?;
    /// assert_eq!(stream.ttl()?, 100);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn ttl(&self) -> io::Result<u32> {
        self.syscall.tcp_stream_ttl(&self.sys_socket)
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_ttl(100)?;
    /// assert_eq!(stream.ttl()?, 100);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.syscall.tcp_stream_set_ttl(&self.sys_socket, ttl)
    }

    /// Gets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// For more information about this option, see [`set_nodelay`].
    ///
    /// [`set_nodelay`]: #method.set_nodelay
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_nodelay(true)?;
    /// assert_eq!(stream.nodelay()?, true);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn nodelay(&self) -> io::Result<bool> {
        self.syscall.tcp_stream_nodelay(&self.sys_socket)
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// If set, this option disables the Nagle algorithm. This means that
    /// segments are always sent as soon as possible, even if there is only a
    /// small amount of data. When not set, data is buffered until there is a
    /// sufficient amount to send out, thereby avoiding the frequent sending of
    /// small packets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_nodelay(true)?;
    /// assert_eq!(stream.nodelay()?, true);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.syscall
            .tcp_stream_set_nodelay(&self.sys_socket, nodelay)
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This method will cause all pending and future I/O on the specified portions to return
    /// immediately with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use std::net::Shutdown;
    ///
    /// use rasi::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// stream.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: std::net::Shutdown) -> std::io::Result<()> {
        self.syscall.tcp_stream_shutdown(&self.sys_socket, how)
    }

    /// Helper method for splitting `TcpStream` object into two halves.
    ///
    /// The two halves returned implement the AsyncRead and AsyncWrite traits, respectively.
    pub fn split(self) -> (TcpStreamRead, TcpStreamWrite) {
        let sys_socket = Arc::new(self.sys_socket);
        (
            TcpStreamRead {
                sys_socket: sys_socket.clone(),
                syscall: self.syscall,
                read_cancel_handle: self.read_cancel_handle,
            },
            TcpStreamWrite {
                sys_socket,
                syscall: self.syscall,
                write_cancel_handle: self.write_cancel_handle,
            },
        )
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match self
            .syscall
            .tcp_stream_read(cx.waker().clone(), &self.sys_socket, buf)
        {
            rasi_syscall::CancelablePoll::Ready(r) => {
                log::trace!("{:?} read {:?}", self, r);
                Poll::Ready(r)
            }
            rasi_syscall::CancelablePoll::Pending(read_cancel_handle) => {
                log::trace!("{:?} read pending", self);
                self.read_cancel_handle = read_cancel_handle;
                Poll::Pending
            }
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self
            .syscall
            .tcp_stream_write(cx.waker().clone(), &self.sys_socket, buf)
        {
            rasi_syscall::CancelablePoll::Ready(r) => {
                log::trace!("{:?} write {:?}", self, r);
                Poll::Ready(r)
            }
            rasi_syscall::CancelablePoll::Pending(write_cancel_handle) => {
                log::trace!("{:?} write pending", self,);
                self.write_cancel_handle = write_cancel_handle;
                Poll::Pending
            }
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        self.shutdown(std::net::Shutdown::Both)?;
        Poll::Ready(Ok(()))
    }
}

/// The read half of [`TcpStream`] created by [`split`](TcpStream::split) function.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
/// #
/// use rasi::net::TcpStream;
///
/// let stream = TcpStream::connect("127.0.0.1:8080").await?;
/// let (read,write) = stream.split();
/// #
/// # Ok(()) }) }
/// ```
pub struct TcpStreamRead {
    /// syscall socket handle.
    sys_socket: Arc<rasi_syscall::Handle>,

    /// a reference to syscall interface .
    syscall: &'static dyn Network,

    /// The cancel handle reference to latest pending read ops.
    read_cancel_handle: Option<Handle>,
}

impl AsyncRead for TcpStreamRead {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match self
            .syscall
            .tcp_stream_read(cx.waker().clone(), &self.sys_socket, buf)
        {
            rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
            rasi_syscall::CancelablePoll::Pending(read_cancel_handle) => {
                self.read_cancel_handle = read_cancel_handle;
                Poll::Pending
            }
        }
    }
}

/// The write half of [`TcpStream`] created by [`split`](TcpStream::split) function.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
/// #
/// use rasi::net::TcpStream;
///
/// let stream = TcpStream::connect("127.0.0.1:8080").await?;
/// let (read,write) = stream.split();
/// #
/// # Ok(()) }) }
/// ```
pub struct TcpStreamWrite {
    /// syscall socket handle.
    sys_socket: Arc<rasi_syscall::Handle>,

    /// a reference to syscall interface .
    syscall: &'static dyn Network,

    /// The cancel handle reference to latest pending write ops.
    write_cancel_handle: Option<Handle>,
}

impl AsyncWrite for TcpStreamWrite {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self
            .syscall
            .tcp_stream_write(cx.waker().clone(), &self.sys_socket, buf)
        {
            rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
            rasi_syscall::CancelablePoll::Pending(write_cancel_handle) => {
                self.write_cancel_handle = write_cancel_handle;
                Poll::Pending
            }
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        self.syscall
            .tcp_stream_shutdown(&self.sys_socket, std::net::Shutdown::Both)?;

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
/// ## Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
/// #
/// use rasi::net::UdpSocket;
///
/// let socket = UdpSocket::bind("127.0.0.1:8080").await?;
/// let mut buf = vec![0u8; 1024];
///
/// loop {
///     let (n, peer) = socket.recv_from(&mut buf).await?;
///     socket.send_to(&buf[..n], peer).await?;
/// }
/// #
/// # }) }
/// ```
pub struct UdpSocket {
    /// syscall socket handle.
    sys_socket: rasi_syscall::Handle,

    /// a reference to syscall interface .
    syscall: &'static dyn Network,
}

impl Display for UdpSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UdpSocket({:?})", self.local_addr())
    }
}

impl UdpSocket {
    /// Use global register syscall interface [`Network`] to create a UDP socket from the given address.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this socket. The
    /// port allocated can be queried via the [`local_addr`] method.
    ///
    /// [`local_addr`]: #method.local_addr
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn bind<A: ToSocketAddrs>(laddrs: A) -> io::Result<Self> {
        Self::bind_with(laddrs, global_network()).await
    }
    /// Use custom syscall interface [`Network`] to create a UDP socket from the given address.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this socket. The
    /// port allocated can be queried via the [`local_addr`] method.
    ///
    /// [`local_addr`]: #method.local_addr
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn bind_with<A: ToSocketAddrs>(
        laddrs: A,
        syscall: &'static dyn Network,
    ) -> io::Result<Self> {
        let laddrs = laddrs.to_socket_addrs()?.collect::<Vec<_>>();

        let sys_socket =
            cancelable_would_block(|cx| syscall.udp_bind(cx.waker().clone(), &laddrs)).await?;

        Ok(UdpSocket {
            sys_socket,
            syscall,
        })
    }
    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out which port was
    /// actually bound.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    /// let addr = socket.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.syscall.udp_local_addr(&self.sys_socket)
    }

    /// Sends data on the socket to the given address.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::UdpSocket;
    ///
    /// const THE_MERCHANT_OF_VENICE: &[u8] = b"
    ///     If you prick us, do we not bleed?
    ///     If you tickle us, do we not laugh?
    ///     If you poison us, do we not die?
    ///     And if you wrong us, shall we not revenge?
    /// ";
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    ///
    /// let addr = "127.0.0.1:7878".parse().unwrap();
    /// let sent = socket.send_to(THE_MERCHANT_OF_VENICE, addr).await?;
    /// log::trace!("Sent {} bytes to {}", sent, addr);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        cancelable_would_block(|cx| {
            self.syscall
                .udp_send_to(cx.waker().clone(), &self.sys_socket, buf, addr)
        })
        .await
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    ///
    /// let mut buf = vec![0; 1024];
    /// let (n, peer) = socket.recv_from(&mut buf).await?;
    /// log::trace!("Received {} bytes from {}", n, peer);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        cancelable_would_block(|cx| {
            self.syscall
                .udp_recv_from(cx.waker().clone(), &self.sys_socket, buf)
        })
        .await
    }

    /// Gets the value of the `SO_BROADCAST` option for this socket.
    ///
    /// For more information about this option, see [`set_broadcast`].
    ///
    /// [`set_broadcast`]: #method.set_broadcast
    pub fn broadcast(&self) -> io::Result<bool> {
        self.syscall.udp_broadcast(&self.sys_socket)
    }

    /// Sets the value of the `SO_BROADCAST` option for this socket.
    ///
    /// When enabled, this socket is allowed to send packets to a broadcast address.
    pub fn set_broadcast(&self, on: bool) -> io::Result<()> {
        self.syscall.udp_set_broadcast(&self.sys_socket, on)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`].
    ///
    /// [`set_ttl`]: #method.set_ttl
    pub fn ttl(&self) -> io::Result<u32> {
        self.syscall.udp_ttl(&self.sys_socket)
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.syscall.udp_set_ttl(&self.sys_socket, ttl)
    }

    /// Executes an operation of the `IP_ADD_MEMBERSHIP` type.
    ///
    /// This method specifies a new multicast group for this socket to join. The address must be
    /// a valid multicast address, and `interface` is the address of the local interface with which
    /// the system should join the multicast group. If it's equal to `INADDR_ANY` then an
    /// appropriate interface is chosen by the system.
    pub fn join_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> io::Result<()> {
        self.syscall
            .udp_join_multicast_v4(&self.sys_socket, multiaddr, interface)
    }

    /// Executes an operation of the `IPV6_ADD_MEMBERSHIP` type.
    ///
    /// This function specifies a new multicast group for this socket to join.
    /// The address must be a valid multicast address, and `interface` is the
    /// index of the interface to join/leave (or 0 to indicate any interface).
    pub fn join_multicast_v6(&self, multiaddr: Ipv6Addr, interface: u32) -> io::Result<()> {
        self.syscall
            .udp_join_multicast_v6(&self.sys_socket, &multiaddr, interface)
    }

    /// Executes an operation of the `IP_DROP_MEMBERSHIP` type.
    ///
    /// For more information about this option, see
    /// [`join_multicast_v4`][link].
    ///
    /// [link]: #method.join_multicast_v4
    pub fn leave_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> io::Result<()> {
        self.syscall
            .udp_leave_multicast_v4(&self.sys_socket, multiaddr, interface)
    }

    /// Executes an operation of the `IPV6_DROP_MEMBERSHIP` type.
    ///
    /// For more information about this option, see
    /// [`join_multicast_v6`][link].
    ///
    /// [link]: #method.join_multicast_v6
    pub fn leave_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> io::Result<()> {
        self.syscall
            .udp_leave_multicast_v6(&self.sys_socket, multiaddr, interface)
    }
}

#[cfg(all(unix, feature = "unix_socket"))]
mod unix {
    use std::{fmt::Debug, io, os::unix::net::SocketAddr, path::Path, task::Poll};

    use futures::{AsyncRead, AsyncWrite};
    use rasi_syscall::{global_network, Handle, Network};

    use crate::utils::cancelable_would_block;

    /// A Unix socket server, listening for connections.
    /// After creating a UnixListener by binding it to a socket address,
    /// it listens for incoming Unix connections. These can be accepted
    /// by awaiting elements from the async stream of incoming connections.
    ///
    /// The socket will be closed when the value is dropped.
    /// The Transmission Control Protocol is specified in IETF RFC 793.
    /// This type is an async version of [`std::os::unix::net::UnixListener`].
    pub struct UnixListener {
        /// syscall socket handle.
        sys_socket: rasi_syscall::Handle,

        /// a reference to syscall interface .
        syscall: &'static dyn Network,
    }

    impl UnixListener {
        /// Creates a new UnixListener with custom [syscall](rasi_syscall::Network) which will be bound to the specified address.
        ///
        /// See [`bind`](UnixListener::bind) for more information.
        pub async fn bind_with<P: AsRef<Path>>(
            path: P,
            syscall: &'static dyn Network,
        ) -> io::Result<Self> {
            let path = path.as_ref();

            let sys_socket = cancelable_would_block(move |cx| {
                syscall.unix_listener_bind(cx.waker().clone(), path)
            })
            .await?;

            Ok(UnixListener {
                sys_socket,
                syscall,
            })
        }

        /// Creates a new TcpListener with global [syscall](rasi_syscall::Network) which will be bound to the specified address.
        ///
        /// Use function [`bind`](UnixListener::bind_with) to specified custom [syscall](rasi_syscall::Network)
        ///
        /// # Examples
        /// Create a unix socket listener
        ///
        /// ```no_run
        /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
        /// #
        /// use rasi::net::UnixListener;
        ///
        /// let listener = UnixListener::bind("/tmp/sock").await?;
        /// #
        /// # Ok(()) }) }
        /// ```
        pub async fn bind<P: AsRef<Path>>(path: P) -> io::Result<Self> {
            Self::bind_with(path, global_network()).await
        }

        /// Accepts a new incoming connection to this listener.
        ///
        /// When a connection is established, the corresponding stream and address will be returned.
        ///
        /// ## Examples
        ///
        /// ```no_run
        /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
        /// #
        /// use rasi::net::UnixListener;
        ///
        /// let listener = UnixListener::bind("/tmp/sock").await?;
        /// let (stream, addr) = listener.accept().await?;
        /// #
        /// # Ok(()) }) }
        pub async fn accept(&self) -> io::Result<(UnixStream, SocketAddr)> {
            let (sys_socket, raddr) = cancelable_would_block(|cx| {
                self.syscall
                    .unix_listener_accept(cx.waker().clone(), &self.sys_socket)
            })
            .await?;

            Ok((UnixStream::new(sys_socket, self.syscall), raddr))
        }

        /// Returns the local address that this listener is bound to.
        ///
        /// This can be useful, for example, to identify when binding to port 0 which port was assigned
        /// by the OS.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
        /// #
        /// use rasi::net::TcpListener;
        ///
        /// let listener = TcpListener::bind("127.0.0.1:8080").await?;
        /// let addr = listener.local_addr()?;
        /// #
        /// # Ok(()) }) }
        /// ```
        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.syscall.unix_listener_local_addr(&self.sys_socket)
        }
    }

    pub struct UnixStream {
        /// syscall socket handle.
        sys_socket: rasi_syscall::Handle,

        /// a reference to syscall interface .
        syscall: &'static dyn Network,

        /// The cancel handle reference to latest pending write ops.
        write_cancel_handle: Option<Handle>,

        /// The cancel handle reference to latest pending read ops.
        read_cancel_handle: Option<Handle>,
    }

    impl Debug for UnixStream {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("UnixStream")
                .field("socket", &self.sys_socket)
                .field(
                    "syscall",
                    &format!("{:?}", self.syscall as *const dyn Network),
                )
                .finish()
        }
    }

    impl UnixStream {
        fn new(sys_socket: rasi_syscall::Handle, syscall: &'static dyn Network) -> Self {
            Self {
                sys_socket,
                syscall,
                write_cancel_handle: None,
                read_cancel_handle: None,
            }
        }

        /// Using custom [`syscall`](rasi_syscall::Network) interface to create a new TCP
        /// stream connected to the specified address.
        ///
        /// see [`connect`](UnixStream::connect) for more information.
        pub async fn connect_with<P: AsRef<Path>>(
            path: P,
            syscall: &'static dyn Network,
        ) -> io::Result<Self> {
            let path = path.as_ref();

            let sys_socket =
                cancelable_would_block(|cx| syscall.unix_stream_connect(cx.waker().clone(), path))
                    .await?;

            Ok(Self::new(sys_socket, syscall))
        }

        /// Using global [`syscall`](rasi_syscall::Network) interface to create a new TCP
        /// stream connected to the specified address.
        ///
        /// This method will create a new Unix socket and attempt to connect it to the `path`
        /// provided. The [returned future] will be resolved once the stream has successfully
        /// connected, or it will return an error if one occurs.
        ///
        /// [returned future]: struct.Connect.html
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
        /// #
        /// use rasi::net::UnixStream;
        ///
        /// let stream = UnixStream::connect("/tmp/socket").await?;
        /// #
        /// # Ok(()) }) }
        /// ```
        pub async fn connect<P: AsRef<Path>>(path: P) -> io::Result<Self> {
            Self::connect_with(path, global_network()).await
        }

        /// Returns the local address that this stream is connected to.
        ///
        /// ## Examples
        ///
        /// ```no_run
        /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
        /// #
        /// use rasi::net::UnixStream;
        ///
        /// let stream = UnixStream::connect("/tmp/socket").await?;
        /// let addr = stream.local_addr()?;
        /// #
        /// # Ok(()) }) }
        /// ```
        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.syscall.unix_stream_local_addr(&self.sys_socket)
        }

        /// Returns the remote address that this stream is connected to.
        ///
        /// ## Examples
        ///
        /// ```no_run
        /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
        /// #
        /// use rasi::net::TcpStream;
        ///
        /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
        /// let peer = stream.peer_addr()?;
        /// #
        /// # Ok(()) }) }
        /// ```
        pub fn peer_addr(&self) -> io::Result<SocketAddr> {
            self.syscall.unix_stream_peer_addr(&self.sys_socket)
        }
    }

    impl AsyncRead for UnixStream {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut [u8],
        ) -> std::task::Poll<io::Result<usize>> {
            match self
                .syscall
                .unix_stream_read(cx.waker().clone(), &self.sys_socket, buf)
            {
                rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
                rasi_syscall::CancelablePoll::Pending(read_cancel_handle) => {
                    self.read_cancel_handle = read_cancel_handle;
                    Poll::Pending
                }
            }
        }
    }

    impl AsyncWrite for UnixStream {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            match self
                .syscall
                .unix_stream_write(cx.waker().clone(), &self.sys_socket, buf)
            {
                rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
                rasi_syscall::CancelablePoll::Pending(write_cancel_handle) => {
                    self.write_cancel_handle = write_cancel_handle;
                    Poll::Pending
                }
            }
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<io::Result<()>> {
            self.syscall
                .unix_stream_shutdown(&self.sys_socket, std::net::Shutdown::Both)?;

            Poll::Ready(Ok(()))
        }
    }
}

#[cfg(all(unix, feature = "unix_socket"))]
pub use unix::*;
