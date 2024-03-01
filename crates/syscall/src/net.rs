//! syscall for networking.

use std::{
    io,
    net::{IpAddr, Shutdown, SocketAddr},
    sync::OnceLock,
    task::Waker,
};

use crate::{
    cancellable::{Cancelable, CancelablePoll},
    handle::Handle,
};

/// Network-related system call interface
pub trait Network: Cancelable + Sync + Send {
    /// Executes an operation of the IP_ADD_MEMBERSHIP type.
    ///
    /// This function specifies a new multicast group for this socket to join.
    /// The address must be a valid multicast address, and interface is the
    /// address of the local interface with which the system should join the
    /// multicast group. If it’s equal to INADDR_ANY then an appropriate
    /// interface is chosen by the system.
    fn udp_join_multicast(
        &self,
        handle: &Handle,
        multiaddr: &IpAddr,
        interface: &IpAddr,
    ) -> io::Result<()>;

    /// Executes an operation of the IP_DROP_MEMBERSHIP type.
    /// For more information about this option, see [`udp_join_multicast`](Self::udp_join_multicast).
    fn udp_leave_multicast(
        &self,
        handle: &Handle,
        multiaddr: &IpAddr,
        interface: &IpAddr,
    ) -> io::Result<()>;

    /// Sets the value of the SO_BROADCAST option for this socket.
    /// When enabled, this socket is allowed to send packets to a broadcast address.
    fn udp_set_broadcast(&self, handle: &Handle, on: bool) -> io::Result<()>;

    /// Gets the value of the SO_BROADCAST option for this socket.
    /// For more information about this option, see [`udp_set_broadcast`](Self::udp_set_broadcast).
    fn udp_broadcast(&self, handle: &Handle) -> io::Result<bool>;

    /// Gets the value of the IP_TTL option for this socket.
    /// For more information about this option, see [`tcp_listener_set_ttl`](Self::tcp_listener_set_ttl).
    fn udp_ttl(&self, handle: &Handle) -> io::Result<u32>;

    /// Sets the value for the IP_TTL option on this socket.
    /// This value sets the time-to-live field that is used in every packet sent from this socket.
    fn udp_set_ttl(&self, handle: &Handle, ttl: u32) -> io::Result<()>;

    /// Returns the local [`socket address`](SocketAddr) bound to this udp socket.
    fn udp_local_addr(&self, handle: &Handle) -> io::Result<SocketAddr>;

    /// Create udp socket and bind it to `laddrs`.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this socket. The
    /// port allocated can be queried via the [`udp_local_addr`](Self::udp_local_addr) method.
    ///
    /// Returns [`CancelablePoll::Pending(CancelHandle)`](CancelablePoll::Pending),
    /// indicating that the current operation could not be completed
    /// immediately and needs to be retried later.
    fn udp_bind(&self, waker: Waker, laddrs: &[SocketAddr]) -> CancelablePoll<io::Result<Handle>>;

    /// Sends data on the socket to the given `target` address.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// Returns [`CancelablePoll::Pending(CancelHandle)`](CancelablePoll::Pending),
    /// indicating that the current operation could not be completed
    /// immediately and needs to be retried later.
    fn udp_send_to(
        &self,
        waker: Waker,
        socket: &Handle,
        target: SocketAddr,
    ) -> CancelablePoll<io::Result<usize>>;

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the origin.
    ///
    /// Returns [`CancelablePoll::Pending(CancelHandle)`](CancelablePoll::Pending),
    /// indicating that the current operation could not be completed
    /// immediately and needs to be retried later.
    fn udp_recv_from(
        &self,
        waker: Waker,
        socket: &Handle,
    ) -> CancelablePoll<io::Result<(usize, SocketAddr)>>;

    /// Create new `TcpListener` which will be bound to the specified `laddrs`
    ///
    /// The returned listener is ready for accepting connections.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this listener.
    /// The port allocated can be queried via the [`tcp_listener_local_addr`](Self::tcp_listener_local_addr) method.
    ///
    /// Returns [`CancelablePoll::Pending(CancelHandle)`](CancelablePoll::Pending),
    /// indicating that the current operation could not be completed
    /// immediately and needs to be retried later.
    fn tcp_listener_bind(
        &self,
        waker: Waker,
        laddrs: &[SocketAddr],
    ) -> CancelablePoll<io::Result<Handle>>;

    /// Returns the local [`socket address`](SocketAddr) bound to this tcp listener.
    fn tcp_listener_local_addr(&self, handle: &Handle) -> io::Result<SocketAddr>;

    /// Gets the value of the IP_TTL option for this socket.
    /// For more information about this option, see [`tcp_listener_set_ttl`](Self::tcp_listener_set_ttl).
    fn tcp_listener_ttl(&self, handle: &Handle) -> io::Result<u32>;

    /// Sets the value for the IP_TTL option on this socket.
    /// This value sets the time-to-live field that is used in every packet sent from this socket.
    fn tcp_listener_set_ttl(&self, handle: &Handle, ttl: u32) -> io::Result<()>;

    /// Accepts a new incoming connection to this tcp listener.
    ///
    /// When a connection is established, the corresponding stream and address will be returned.
    ///
    /// Returns [`CancelablePoll::Pending(CancelHandle)`](CancelablePoll::Pending),
    /// indicating that the current operation could not be completed
    /// immediately and needs to be retried later.
    fn tcp_listener_accept(
        &self,
        handle: &Handle,
    ) -> CancelablePoll<io::Result<(Handle, SocketAddr)>>;

    /// Create a new `TcpStream` and connect to `raddrs`.
    ///
    /// The port allocated can be queried via the [`tcp_stream_local_addr`](Self::tcp_stream_local_addr) method.
    ///
    /// Returns [`CancelablePoll::Pending(CancelHandle)`](CancelablePoll::Pending),
    /// indicating that the current operation could not be completed
    /// immediately and needs to be retried later.
    fn tcp_stream_connect(&self, raddrs: &[SocketAddr]) -> CancelablePoll<io::Result<()>>;

    /// Returns the local [`socket address`](SocketAddr) bound to this tcp stream.
    fn tcp_stream_local_addr(&self, handle: &Handle) -> io::Result<SocketAddr>;

    /// Returns the remote [`socket address`](SocketAddr) this tcp stream connected.
    fn tcp_stream_remote_addr(&self, handle: &Handle) -> io::Result<SocketAddr>;

    /// Gets the value of the TCP_NODELAY option on this socket.
    /// For more information about this option, see [`tcp_stream_set_nodelay`](Self::tcp_stream_set_nodelay).
    fn tcp_stream_nodelay(&self) -> io::Result<bool>;

    /// Sets the value of the TCP_NODELAY option on this socket.
    ///
    /// If set, this option disables the Nagle algorithm.
    /// This means that segments are always sent as soon as possible,
    /// even if there is only a small amount of data. When not set,
    /// data is buffered until there is a sufficient amount to send out,
    /// thereby avoiding the frequent sending of small packets.
    fn tcp_stream_set_nodelay(&self, nodelay: bool) -> io::Result<()>;

    /// Gets the value of the IP_TTL option for this socket.
    /// For more information about this option, see [`tcp_listener_set_ttl`](Self::tcp_listener_set_ttl).
    fn tcp_stream_ttl(&self, handle: &Handle) -> io::Result<u32>;

    /// Sets the value for the IP_TTL option on this socket.
    /// This value sets the time-to-live field that is used in every packet sent from this socket.
    fn tcp_stream_set_ttl(&self, handle: &Handle, ttl: u32) -> io::Result<()>;

    /// Shuts down the read, write, or both halves of this connection.
    /// This function will cause all pending and future I/O on the specified
    /// portions to return immediately with an appropriate value (see the documentation of Shutdown).
    fn tcp_stream_shutdown(&self, handle: &Handle, how: Shutdown) -> io::Result<()>;
}

static GLOBAL_NETWORK: OnceLock<Box<dyn Network>> = OnceLock::new();

/// Register provided [`Network`] as global network implementation.
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_global_network<E: Network + 'static>(executor: E) {
    if GLOBAL_NETWORK.set(Box::new(executor)).is_err() {
        panic!("Multiple calls to register_global_network are not permitted!!!");
    }
}

/// Get global register [`Network`] syscall interface.
///
/// # Panic
///
/// You should call [`register_global_network`] first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_network first`
pub fn global_network() -> &'static dyn Network {
    GLOBAL_NETWORK
        .get()
        .expect("Call register_global_network first")
        .as_ref()
}
