use std::{
    io::{Error, ErrorKind, Read, Write},
    net::Shutdown,
    ops::Deref,
    os::fd::FromRawFd,
    sync::RwLock,
    task::Poll,
};

use mio::{event::Source, Interest, Token};
use rasi::net::register_network_driver;
use socket2::{Domain, Protocol, Type};

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

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        would_block(self.token, cx.waker().clone(), Interest::WRITABLE, || {
            log::trace!("tcp_connect, poll_ready {:?}", self.token);

            if let Err(err) = self.deref().take_error() {
                return Err(err);
            }

            match self.deref().peer_addr() {
                Ok(_) => {
                    return Ok(());
                }
                Err(err)
                    if err.kind() == ErrorKind::NotConnected
                        || err.raw_os_error() == Some(libc::EINPROGRESS) =>
                {
                    return Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, ""));
                }
                Err(err) => {
                    return Err(err);
                }
            }
        })
    }
}

struct MioUdpSocket {
    mio_socket: MioSocket<mio::net::UdpSocket>,
    shutdown: RwLock<(bool, bool)>,
}

impl rasi::net::syscall::DriverUdpSocket for MioUdpSocket {
    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.mio_socket.socket.local_addr()
    }

    fn peer_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.mio_socket.socket.peer_addr()
    }

    fn ttl(&self) -> std::io::Result<u32> {
        self.mio_socket.socket.ttl()
    }

    fn set_ttl(&self, ttl: u32) -> std::io::Result<()> {
        self.mio_socket.socket.set_ttl(ttl)
    }

    fn join_multicast_v4(
        &self,
        multiaddr: &std::net::Ipv4Addr,
        interface: &std::net::Ipv4Addr,
    ) -> std::io::Result<()> {
        self.mio_socket
            .socket
            .join_multicast_v4(multiaddr, interface)
    }

    fn join_multicast_v6(
        &self,
        multiaddr: &std::net::Ipv6Addr,
        interface: u32,
    ) -> std::io::Result<()> {
        self.mio_socket
            .socket
            .join_multicast_v6(multiaddr, interface)
    }

    fn leave_multicast_v4(
        &self,
        multiaddr: &std::net::Ipv4Addr,
        interface: &std::net::Ipv4Addr,
    ) -> std::io::Result<()> {
        self.mio_socket
            .socket
            .leave_multicast_v4(multiaddr, interface)
    }

    fn leave_multicast_v6(
        &self,
        multiaddr: &std::net::Ipv6Addr,
        interface: u32,
    ) -> std::io::Result<()> {
        self.mio_socket
            .socket
            .leave_multicast_v6(multiaddr, interface)
    }

    fn set_broadcast(&self, on: bool) -> std::io::Result<()> {
        self.mio_socket.socket.set_broadcast(on)
    }

    fn broadcast(&self) -> std::io::Result<bool> {
        self.mio_socket.socket.broadcast()
    }

    fn poll_recv_from(
        &self,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<(usize, std::net::SocketAddr)>> {
        let shutdown = self.shutdown.read().unwrap();

        if shutdown.0 {
            return Poll::Ready(Err(Error::new(
                ErrorKind::BrokenPipe,
                "UdpSocket read shutdown.",
            )));
        }

        would_block(
            self.mio_socket.token,
            cx.waker().clone(),
            Interest::READABLE,
            || self.mio_socket.socket.recv_from(buf),
        )
    }

    fn poll_send_to(
        &self,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
        peer: std::net::SocketAddr,
    ) -> Poll<std::io::Result<usize>> {
        let shutdown = self.shutdown.read().unwrap();
        if shutdown.1 {
            return Poll::Ready(Err(Error::new(
                ErrorKind::BrokenPipe,
                "UdpSocket write shutdown.",
            )));
        }

        would_block(
            self.mio_socket.token,
            cx.waker().clone(),
            Interest::WRITABLE,
            || self.mio_socket.socket.send_to(buf, peer),
        )
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This method will cause all pending and future I/O on the specified portions to return
    /// immediately with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    fn shutdown(&self, how: Shutdown) -> std::io::Result<()> {
        let mut locker = self.shutdown.write().unwrap();

        match how {
            Shutdown::Read => {
                locker.0 = true;

                global_reactor().notify(self.mio_socket.token, Interest::READABLE);
            }
            Shutdown::Write => {
                locker.1 = true;
                global_reactor().notify(self.mio_socket.token, Interest::WRITABLE);
            }
            Shutdown::Both => {
                locker.0 = true;
                locker.1 = true;
                global_reactor().notify(
                    self.mio_socket.token,
                    Interest::WRITABLE.add(Interest::READABLE),
                );
            }
        }

        Ok(())
    }
}

#[cfg(unix)]
type MioUnixListener = MioSocket<mio::net::UnixListener>;

#[cfg(unix)]
impl rasi::net::syscall::unix::DriverUnixListener for MioUnixListener {
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
impl rasi::net::syscall::unix::DriverUnixStream for MioUnixStream {
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

    fn tcp_connect(&self, raddr: &std::net::SocketAddr) -> std::io::Result<rasi::net::TcpStream> {
        log::trace!("tcp_connect, raddr={}", raddr);

        let mut socket = mio::net::TcpStream::connect(raddr.clone())?;

        let token = Token::next();

        global_reactor().register(
            &mut socket,
            token,
            Interest::READABLE.add(Interest::WRITABLE),
        )?;

        return Ok(MioTcpStream { token, socket }.into());
    }

    fn udp_bind(&self, laddrs: &[std::net::SocketAddr]) -> std::io::Result<rasi::net::UdpSocket> {
        let mut last_error = None;
        for laddr in laddrs {
            match socket2::Socket::new(
                if laddr.is_ipv4() {
                    Domain::IPV4
                } else {
                    Domain::IPV6
                },
                Type::DGRAM,
                Some(Protocol::UDP),
            ) {
                Ok(s) => {
                    s.set_reuse_address(true)?;
                    s.bind(&laddr.clone().into())?;
                    s.set_nonblocking(true)?;

                    #[cfg(unix)]
                    let mut socket = {
                        use std::os::fd::IntoRawFd;

                        unsafe { mio::net::UdpSocket::from_raw_fd(s.into_raw_fd()) }
                    };

                    #[cfg(windows)]
                    let mut socket = {
                        use std::os::windows::io::IntoRawSocket;

                        unsafe { mio::net::UdpSocket::from_raw_socket(s.into_raw_socket()) }
                    };

                    let token = Token::next();

                    global_reactor().register(
                        &mut socket,
                        token,
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;

                    return Ok(MioUdpSocket {
                        mio_socket: MioSocket { socket, token },
                        shutdown: RwLock::new((false, false)),
                    }
                    .into());
                }
                Err(err) => {
                    last_error = Some(err);
                    continue;
                }
            }
        }

        return Err(last_error.unwrap_or(Error::new(ErrorKind::InvalidInput, "Empty laddrs")));
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

/// This function using [`register_network_driver`] to register the `MioNetwork` to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_network_driver`)
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
