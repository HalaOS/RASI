//! The implementation of [`Network`] syscall.

use rasi_syscall::Network;

/// This type implements the system call [`network`] using the underlying [`mio`].
pub struct MioNetwork {}

impl Network for MioNetwork {
    fn udp_join_multicast(
        &self,
        handle: &rasi_syscall::Handle,
        multiaddr: &std::net::IpAddr,
        interface: &std::net::IpAddr,
    ) -> std::io::Result<()> {
        match (multiaddr, interface) {
            (std::net::IpAddr::V4(_), std::net::IpAddr::V4(_)) => todo!(),
            (std::net::IpAddr::V6(_), std::net::IpAddr::V6(_)) => todo!(),
            _ => {}
        }
        todo!()
    }

    fn udp_leave_multicast(
        &self,
        handle: &rasi_syscall::Handle,
        multiaddr: &std::net::IpAddr,
        interface: &std::net::IpAddr,
    ) -> std::io::Result<()> {
        todo!()
    }

    fn udp_set_broadcast(&self, handle: &rasi_syscall::Handle, on: bool) -> std::io::Result<()> {
        todo!()
    }

    fn udp_broadcast(&self, handle: &rasi_syscall::Handle) -> std::io::Result<bool> {
        todo!()
    }

    fn udp_ttl(&self, handle: &rasi_syscall::Handle) -> std::io::Result<u32> {
        todo!()
    }

    fn udp_set_ttl(&self, handle: &rasi_syscall::Handle, ttl: u32) -> std::io::Result<()> {
        todo!()
    }

    fn udp_local_addr(
        &self,
        handle: &rasi_syscall::Handle,
    ) -> std::io::Result<std::net::SocketAddr> {
        todo!()
    }

    fn udp_bind(
        &self,
        waker: std::task::Waker,
        laddrs: &[std::net::SocketAddr],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
        todo!()
    }

    fn udp_send_to(
        &self,
        waker: std::task::Waker,
        socket: &rasi_syscall::Handle,
        buf: &[u8],
        target: std::net::SocketAddr,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
        todo!()
    }

    fn udp_recv_from(
        &self,
        waker: std::task::Waker,
        socket: &rasi_syscall::Handle,
        buf: &mut [u8],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<(usize, std::net::SocketAddr)>> {
        todo!()
    }

    fn tcp_listener_bind(
        &self,
        waker: std::task::Waker,
        laddrs: &[std::net::SocketAddr],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
        todo!()
    }

    fn tcp_listener_local_addr(
        &self,
        handle: &rasi_syscall::Handle,
    ) -> std::io::Result<std::net::SocketAddr> {
        todo!()
    }

    fn tcp_listener_ttl(&self, handle: &rasi_syscall::Handle) -> std::io::Result<u32> {
        todo!()
    }

    fn tcp_listener_set_ttl(&self, handle: &rasi_syscall::Handle, ttl: u32) -> std::io::Result<()> {
        todo!()
    }

    fn tcp_listener_accept(
        &self,
        waker: std::task::Waker,
        handle: &rasi_syscall::Handle,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<(rasi_syscall::Handle, std::net::SocketAddr)>>
    {
        todo!()
    }

    fn tcp_stream_connect(
        &self,
        waker: std::task::Waker,
        raddrs: &[std::net::SocketAddr],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
        todo!()
    }

    fn tcp_stream_write(
        &self,
        waker: std::task::Waker,
        socket: &rasi_syscall::Handle,
        buf: &[u8],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
        todo!()
    }

    fn tcp_stream_read(
        &self,
        waker: std::task::Waker,
        socket: &rasi_syscall::Handle,
        buf: &mut [u8],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
        todo!()
    }

    fn tcp_stream_local_addr(
        &self,
        handle: &rasi_syscall::Handle,
    ) -> std::io::Result<std::net::SocketAddr> {
        todo!()
    }

    fn tcp_stream_remote_addr(
        &self,
        handle: &rasi_syscall::Handle,
    ) -> std::io::Result<std::net::SocketAddr> {
        todo!()
    }

    fn tcp_stream_nodelay(&self, handle: &rasi_syscall::Handle) -> std::io::Result<bool> {
        todo!()
    }

    fn tcp_stream_set_nodelay(
        &self,
        handle: &rasi_syscall::Handle,
        nodelay: bool,
    ) -> std::io::Result<()> {
        todo!()
    }

    fn tcp_stream_ttl(&self, handle: &rasi_syscall::Handle) -> std::io::Result<u32> {
        todo!()
    }

    fn tcp_stream_set_ttl(&self, handle: &rasi_syscall::Handle, ttl: u32) -> std::io::Result<()> {
        todo!()
    }

    fn tcp_stream_shutdown(
        &self,
        handle: &rasi_syscall::Handle,
        how: std::net::Shutdown,
    ) -> std::io::Result<()> {
        todo!()
    }
}
