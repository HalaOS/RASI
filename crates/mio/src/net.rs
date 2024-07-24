use std::{
    io::{Read, Write},
    ops::Deref,
    task::Poll,
};

use mio::{event::Source, Interest, Token};
use rasi::net::register_network_driver;

use crate::{reactor::global_reactor, token::TokenSequence, utils::would_block};

/// A wrapper of mio event source.
#[derive(Debug)]
pub(crate) struct MioSocket<S: Source> {
    /// Associcated token.
    pub(crate) token: Token,
    /// net source type.
    pub(crate) socket: S,
}

impl<S: Source> From<(Token, S)> for MioSocket<S> {
    fn from(value: (Token, S)) -> Self {
        Self {
            token: value.0,
            socket: value.1,
        }
    }
}

impl<S: Source> Deref for MioSocket<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.socket
    }
}

impl<S: Source> Drop for MioSocket<S> {
    fn drop(&mut self) {
        if global_reactor().deregister(&mut self.socket).is_err() {}
    }
}

type MioTcpListener = MioSocket<mio::net::TcpListener>;

impl rasi::net::syscall::DriverTcpListener for MioTcpListener {
    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.socket.local_addr()
    }

    fn ttl(&self) -> std::io::Result<u32> {
        self.socket.ttl()
    }

    fn set_ttl(&self, ttl: u32) -> std::io::Result<()> {
        self.socket.set_ttl(ttl)
    }

    fn poll_next(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<(rasi::net::TcpStream, std::net::SocketAddr)>> {
        would_block(
            self.token,
            cx.waker().clone(),
            Interest::READABLE,
            || match self.socket.accept() {
                Ok((mut stream, raddr)) => {
                    let token = Token::next();

                    global_reactor().register(
                        &mut stream,
                        token,
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;

                    Ok((
                        MioTcpStream {
                            token,
                            socket: stream,
                        }
                        .into(),
                        raddr,
                    ))
                }
                Err(err) => Err(err),
            },
        )
    }
}

type MioTcpStream = MioSocket<mio::net::TcpStream>;

impl rasi::net::syscall::DriverTcpStream for MioTcpStream {
    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.socket.local_addr()
    }

    fn peer_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.socket.peer_addr()
    }

    fn ttl(&self) -> std::io::Result<u32> {
        self.socket.ttl()
    }

    fn set_ttl(&self, ttl: u32) -> std::io::Result<()> {
        self.socket.set_ttl(ttl)
    }

    fn nodelay(&self) -> std::io::Result<bool> {
        self.socket.nodelay()
    }

    fn set_nodelay(&self, nodelay: bool) -> std::io::Result<()> {
        self.socket.set_nodelay(nodelay)
    }

    fn shutdown(&self, how: std::net::Shutdown) -> std::io::Result<()> {
        self.socket.shutdown(how)
    }

    fn poll_read(
        &self,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        would_block(self.token, cx.waker().clone(), Interest::READABLE, || {
            self.deref().read(buf)
        })
    }

    fn poll_write(
        &self,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        would_block(self.token, cx.waker().clone(), Interest::WRITABLE, || {
            self.deref().write(buf)
        })
    }

    fn poll_ready(&self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

type MioUdpSocket = MioSocket<mio::net::UdpSocket>;

impl rasi::net::NDUdpSocket for MioUdpSocket {
    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.socket.local_addr()
    }

    fn peer_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.socket.peer_addr()
    }

    fn ttl(&self) -> std::io::Result<u32> {
        self.socket.ttl()
    }

    fn set_ttl(&self, ttl: u32) -> std::io::Result<()> {
        self.socket.set_ttl(ttl)
    }

    fn join_multicast_v4(
        &self,
        multiaddr: &std::net::Ipv4Addr,
        interface: &std::net::Ipv4Addr,
    ) -> std::io::Result<()> {
        self.socket.join_multicast_v4(multiaddr, interface)
    }

    fn join_multicast_v6(
        &self,
        multiaddr: &std::net::Ipv6Addr,
        interface: u32,
    ) -> std::io::Result<()> {
        self.socket.join_multicast_v6(multiaddr, interface)
    }

    fn leave_multicast_v4(
        &self,
        multiaddr: &std::net::Ipv4Addr,
        interface: &std::net::Ipv4Addr,
    ) -> std::io::Result<()> {
        self.socket.leave_multicast_v4(multiaddr, interface)
    }

    fn leave_multicast_v6(
        &self,
        multiaddr: &std::net::Ipv6Addr,
        interface: u32,
    ) -> std::io::Result<()> {
        self.socket.leave_multicast_v6(multiaddr, interface)
    }

    fn set_broadcast(&self, on: bool) -> std::io::Result<()> {
        self.socket.set_broadcast(on)
    }

    fn broadcast(&self) -> std::io::Result<bool> {
        self.socket.broadcast()
    }

    fn poll_recv_from(
        &self,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<(usize, std::net::SocketAddr)>> {
        would_block(self.token, cx.waker().clone(), Interest::READABLE, || {
            self.socket.recv_from(buf)
        })
    }

    fn poll_send_to(
        &self,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
        peer: std::net::SocketAddr,
    ) -> Poll<std::io::Result<usize>> {
        would_block(self.token, cx.waker().clone(), Interest::WRITABLE, || {
            self.socket.send_to(buf, peer)
        })
    }
}

#[cfg(unix)]
type MioUnixListener = MioSocket<mio::net::UnixListener>;

#[cfg(unix)]
impl rasi::net::unix::NDUnixListener for MioUnixListener {
    fn local_addr(&self) -> std::io::Result<std::os::unix::net::SocketAddr> {
        self.socket.local_addr()
    }

    fn poll_next(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<(rasi::net::unix::UnixStream, std::os::unix::net::SocketAddr)>> {
        would_block(
            self.token,
            cx.waker().clone(),
            Interest::READABLE,
            || match self.socket.accept() {
                Ok((mut stream, raddr)) => {
                    let token = Token::next();

                    global_reactor().register(
                        &mut stream,
                        token,
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;

                    Ok((
                        MioUnixStream {
                            token,
                            socket: stream,
                        }
                        .into(),
                        raddr,
                    ))
                }
                Err(err) => Err(err),
            },
        )
    }
}

#[cfg(unix)]
type MioUnixStream = MioSocket<mio::net::UnixStream>;

#[cfg(unix)]
impl rasi::net::unix::NDUnixStream for MioUnixStream {
    fn shutdown(&self, how: std::net::Shutdown) -> std::io::Result<()> {
        self.socket.shutdown(how)
    }

    fn poll_read(
        &self,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        would_block(self.token, cx.waker().clone(), Interest::READABLE, || {
            self.deref().read(buf)
        })
    }

    fn poll_write(
        &self,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        would_block(self.token, cx.waker().clone(), Interest::WRITABLE, || {
            self.deref().write(buf)
        })
    }

    fn poll_ready(&self, _cx: &mut std::task::Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn local_addr(&self) -> std::io::Result<std::os::unix::net::SocketAddr> {
        self.socket.local_addr()
    }

    fn peer_addr(&self) -> std::io::Result<std::os::unix::net::SocketAddr> {
        self.socket.peer_addr()
    }
}

struct MioNetworkDriver;

impl rasi::net::syscall::Driver for MioNetworkDriver {
    fn tcp_listen(
        &self,
        laddrs: &[std::net::SocketAddr],
    ) -> std::io::Result<rasi::net::TcpListener> {
        let std_socket = std::net::TcpListener::bind(laddrs)?;

        std_socket.set_nonblocking(true)?;

        let mut socket = mio::net::TcpListener::from_std(std_socket);
        let token = Token::next();

        global_reactor().register(
            &mut socket,
            token,
            Interest::READABLE.add(Interest::WRITABLE),
        )?;

        Ok(MioTcpListener { token, socket }.into())
    }

    fn tcp_connect(
        &self,
        raddrs: &[std::net::SocketAddr],
    ) -> std::io::Result<rasi::net::TcpStream> {
        let std_socket = std::net::TcpStream::connect(raddrs)?;

        std_socket.set_nonblocking(true)?;

        let mut socket = mio::net::TcpStream::from_std(std_socket);
        let token = Token::next();

        global_reactor().register(
            &mut socket,
            token,
            Interest::READABLE.add(Interest::WRITABLE),
        )?;

        Ok(MioTcpStream { token, socket }.into())
    }

    fn udp_bind(&self, laddrs: &[std::net::SocketAddr]) -> std::io::Result<rasi::net::UdpSocket> {
        let std_socket = std::net::UdpSocket::bind(laddrs)?;

        std_socket.set_nonblocking(true)?;

        let mut socket = mio::net::UdpSocket::from_std(std_socket);
        let token = Token::next();

        global_reactor().register(
            &mut socket,
            token,
            Interest::READABLE.add(Interest::WRITABLE),
        )?;

        Ok(MioUdpSocket { socket, token }.into())
    }

    #[cfg(unix)]
    fn unix_listen(
        &self,
        path: &std::path::Path,
    ) -> std::io::Result<rasi::net::unix::UnixListener> {
        let mut socket = mio::net::UnixListener::bind(path)?;

        let token = Token::next();

        global_reactor().register(
            &mut socket,
            token,
            Interest::READABLE.add(Interest::WRITABLE),
        )?;

        Ok(MioUnixListener { token, socket }.into())
    }

    #[cfg(unix)]
    fn unix_connect(&self, path: &std::path::Path) -> std::io::Result<rasi::net::unix::UnixStream> {
        let mut socket = mio::net::UnixStream::connect(path)?;

        let token = Token::next();

        global_reactor().register(
            &mut socket,
            token,
            Interest::READABLE.add(Interest::WRITABLE),
        )?;

        Ok(MioUnixStream { token, socket }.into())
    }
}

/// This function using [`register_global_network`] to register the [`MioNetwork`] to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_global_network`)
pub fn register_mio_network() {
    register_network_driver(MioNetworkDriver)
}

#[cfg(test)]
mod tests {

    use rasi_spec::network::run_network_spec;

    use super::*;

    #[futures_test::test]
    async fn test_network() {
        static DRIVER: MioNetworkDriver = MioNetworkDriver;

        run_network_spec(&DRIVER).await;

        #[cfg(unix)]
        rasi_spec::ipc::run_ipc_spec(&DRIVER).await;
    }
}
