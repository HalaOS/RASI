//! The implementation of [`Network`] syscall.

use std::{
    io::{self, Read, Write},
    ops::Deref,
};

use mio::{
    net::{TcpListener, TcpStream, UdpSocket},
    Interest, Token,
};
use rasi_syscall::{ready, register_global_network, Handle, Network};

use crate::{
    reactor::{global_reactor, would_block, MioSocket},
    TokenSequence,
};

/// This type implements the system call [`Network`] using the underlying [`mio`].
#[derive(Default)]
pub struct MioNetwork {}

impl Network for MioNetwork {
    fn udp_join_multicast_v4(
        &self,
        handle: &rasi_syscall::Handle,
        multiaddr: &std::net::Ipv4Addr,
        interface: &std::net::Ipv4Addr,
    ) -> io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.join_multicast_v4(multiaddr, interface)
    }

    fn udp_join_multicast_v6(
        &self,
        handle: &rasi_syscall::Handle,
        multiaddr: &std::net::Ipv6Addr,
        interface: u32,
    ) -> io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.join_multicast_v6(multiaddr, interface)
    }

    fn udp_leave_multicast_v4(
        &self,
        handle: &rasi_syscall::Handle,
        multiaddr: &std::net::Ipv4Addr,
        interface: &std::net::Ipv4Addr,
    ) -> io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.leave_multicast_v4(multiaddr, interface)
    }

    fn udp_leave_multicast_v6(
        &self,
        handle: &rasi_syscall::Handle,
        multiaddr: &std::net::Ipv6Addr,
        interface: u32,
    ) -> io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.leave_multicast_v6(multiaddr, interface)
    }
    fn udp_set_broadcast(&self, handle: &rasi_syscall::Handle, on: bool) -> std::io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.set_broadcast(on)
    }

    fn udp_broadcast(&self, handle: &rasi_syscall::Handle) -> std::io::Result<bool> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.broadcast()
    }

    fn udp_ttl(&self, handle: &rasi_syscall::Handle) -> std::io::Result<u32> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.ttl()
    }

    fn udp_set_ttl(&self, handle: &rasi_syscall::Handle, ttl: u32) -> std::io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.set_ttl(ttl)
    }

    fn udp_local_addr(
        &self,
        handle: &rasi_syscall::Handle,
    ) -> std::io::Result<std::net::SocketAddr> {
        let socket = handle
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket");

        socket.local_addr()
    }

    fn udp_bind(
        &self,
        _waker: std::task::Waker,
        laddrs: &[std::net::SocketAddr],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
        ready(|| {
            let std_socket = std::net::UdpSocket::bind(laddrs)?;

            std_socket.set_nonblocking(true)?;

            let mut socket = UdpSocket::from_std(std_socket);
            let token = Token::next();

            global_reactor().register(
                &mut socket,
                token,
                Interest::READABLE.add(Interest::WRITABLE),
            )?;

            Ok(Handle::new(MioSocket::from((token, socket))))
        })
    }

    fn udp_send_to(
        &self,
        waker: std::task::Waker,
        socket: &rasi_syscall::Handle,
        buf: &[u8],
        target: std::net::SocketAddr,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
        let socket = socket
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket.");

        would_block(socket.token, waker, Interest::WRITABLE, || {
            socket.send_to(buf, target)
        })
    }

    fn udp_recv_from(
        &self,
        waker: std::task::Waker,
        socket: &rasi_syscall::Handle,
        buf: &mut [u8],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<(usize, std::net::SocketAddr)>> {
        let socket = socket
            .downcast::<MioSocket<UdpSocket>>()
            .expect("Expect udpsocket.");

        would_block(socket.token, waker, Interest::READABLE, || {
            socket.recv_from(buf)
        })
    }

    fn tcp_listener_bind(
        &self,
        _waker: std::task::Waker,
        laddrs: &[std::net::SocketAddr],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
        ready(|| {
            let std_socket = std::net::TcpListener::bind(laddrs)?;

            std_socket.set_nonblocking(true)?;

            let mut socket = TcpListener::from_std(std_socket);
            let token = Token::next();

            global_reactor().register(
                &mut socket,
                token,
                Interest::READABLE.add(Interest::WRITABLE),
            )?;

            Ok(Handle::new(MioSocket::from((token, socket))))
        })
    }

    fn tcp_listener_local_addr(
        &self,
        handle: &rasi_syscall::Handle,
    ) -> std::io::Result<std::net::SocketAddr> {
        let socket = handle
            .downcast::<MioSocket<TcpListener>>()
            .expect("Expect TcpListener.");

        socket.local_addr()
    }

    fn tcp_listener_ttl(&self, handle: &rasi_syscall::Handle) -> std::io::Result<u32> {
        let socket = handle
            .downcast::<MioSocket<TcpListener>>()
            .expect("Expect TcpListener.");

        socket.ttl()
    }

    fn tcp_listener_set_ttl(&self, handle: &rasi_syscall::Handle, ttl: u32) -> std::io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<TcpListener>>()
            .expect("Expect TcpListener.");

        socket.set_ttl(ttl)
    }

    fn tcp_listener_accept(
        &self,
        waker: std::task::Waker,
        handle: &rasi_syscall::Handle,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<(rasi_syscall::Handle, std::net::SocketAddr)>>
    {
        let socket = handle
            .downcast::<MioSocket<TcpListener>>()
            .expect("Expect TcpListener.");

        would_block(socket.token, waker, Interest::READABLE, || {
            match socket.accept() {
                Ok((mut stream, raddr)) => {
                    let token = Token::next();

                    global_reactor().register(
                        &mut stream,
                        token,
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;

                    Ok((Handle::new(MioSocket::from((token, stream))), raddr))
                }
                Err(err) => Err(err),
            }
        })
    }

    fn tcp_stream_connect(
        &self,
        _waker: std::task::Waker,
        raddrs: &[std::net::SocketAddr],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
        ready(|| {
            let std_socket = std::net::TcpStream::connect(raddrs)?;

            std_socket.set_nonblocking(true)?;

            let mut socket = TcpStream::from_std(std_socket);
            let token = Token::next();

            global_reactor().register(
                &mut socket,
                token,
                Interest::READABLE.add(Interest::WRITABLE),
            )?;

            Ok(Handle::new(MioSocket::from((token, socket))))
        })
    }

    fn tcp_stream_write(
        &self,
        waker: std::task::Waker,
        socket: &rasi_syscall::Handle,
        buf: &[u8],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
        let socket = socket
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        would_block(socket.token, waker, Interest::WRITABLE, || {
            socket.deref().write(buf)
        })
    }

    fn tcp_stream_read(
        &self,
        waker: std::task::Waker,
        socket: &rasi_syscall::Handle,
        buf: &mut [u8],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
        let socket = socket
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        would_block(socket.token, waker, Interest::READABLE, || {
            socket.deref().read(buf)
        })
    }

    fn tcp_stream_local_addr(
        &self,
        handle: &rasi_syscall::Handle,
    ) -> std::io::Result<std::net::SocketAddr> {
        let socket = handle
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        socket.local_addr()
    }

    fn tcp_stream_remote_addr(
        &self,
        handle: &rasi_syscall::Handle,
    ) -> std::io::Result<std::net::SocketAddr> {
        let socket = handle
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        socket.peer_addr()
    }

    fn tcp_stream_nodelay(&self, handle: &rasi_syscall::Handle) -> std::io::Result<bool> {
        let socket = handle
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        socket.nodelay()
    }

    fn tcp_stream_set_nodelay(
        &self,
        handle: &rasi_syscall::Handle,
        nodelay: bool,
    ) -> std::io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        socket.set_nodelay(nodelay)
    }

    fn tcp_stream_ttl(&self, handle: &rasi_syscall::Handle) -> std::io::Result<u32> {
        let socket = handle
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        socket.ttl()
    }

    fn tcp_stream_set_ttl(&self, handle: &rasi_syscall::Handle, ttl: u32) -> std::io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        socket.set_ttl(ttl)
    }

    fn tcp_stream_shutdown(
        &self,
        handle: &rasi_syscall::Handle,
        how: std::net::Shutdown,
    ) -> std::io::Result<()> {
        let socket = handle
            .downcast::<MioSocket<TcpStream>>()
            .expect("Expect TcpStream.");

        socket.shutdown(how)
    }
}

/// This function using [`register_global_network`] to register the [`MioNetwork`] to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_global_network`)
pub fn register_mio_network() {
    register_global_network(MioNetwork::default())
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use rasi_spec::network::run_network_spec;

    use super::*;

    static INIT: OnceLock<Box<dyn rasi_syscall::Network>> = OnceLock::new();

    fn get_syscall() -> &'static dyn rasi_syscall::Network {
        INIT.get_or_init(|| Box::new(MioNetwork::default()))
            .as_ref()
    }

    #[futures_test::test]
    async fn test_network() {
        run_network_spec(get_syscall()).await;
    }
}
