use std::{
    collections::HashMap,
    future::Future,
    io::{Error, ErrorKind, Result},
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use futures_map::FuturesWaitMap;
use quiche::{Config, RecvInfo};
use rand::{seq::IteratorRandom, thread_rng};
use rasi::{
    net::{get_network_driver, UdpSocket},
    task::spawn_ok,
    timer::TimeoutExt,
};

use futures::StreamExt;
use ring::rand::{SecureRandom, SystemRandom};

use crate::{errors::map_quic_error, QuicConn, QuicConnState, QuicListener};

const MAX_MTU_SIZE: usize = 1600;

#[derive(Debug)]
struct PathInfo {
    /// The specified local address.
    pub from: SocketAddr,
    /// The destination address for the data sent to the peer.
    pub to: SocketAddr,
}

#[derive(Clone)]
struct UdpGroup {
    /// Group sockets pool.
    sockets: Arc<HashMap<SocketAddr, Arc<UdpSocket>>>,
    /// A futures waiting map for [`UdpSocket::recv_from`]
    recv_map: FuturesWaitMap<SocketAddr, Result<(Vec<u8>, SocketAddr)>>,
    /// The recv buf size.
    max_recv_udp_payload_size: u16,
}

impl UdpGroup {
    /// Create new udp group which will be bound to the specified `laddrs`
    pub async fn bind<A: ToSocketAddrs>(laddrs: A, max_recv_udp_payload_size: u16) -> Result<Self> {
        Self::bind_with(laddrs, max_recv_udp_payload_size, get_network_driver()).await
    }

    /// Create new udp group with custom `Driver`.
    pub async fn bind_with<A: ToSocketAddrs>(
        laddrs: A,
        max_recv_udp_payload_size: u16,
        driver: &dyn rasi::net::syscall::Driver,
    ) -> Result<Self> {
        let laddrs = laddrs.to_socket_addrs()?.collect::<Vec<_>>();

        let mut sockets = HashMap::new();

        let recv_map = FuturesWaitMap::new();

        for laddr in laddrs {
            let socket = Arc::new(UdpSocket::bind_with([laddr].as_slice(), driver).await?);

            // get the port allocated by OS.
            let laddr = socket.local_addr()?;

            // Init recv_from loop.
            recv_map.insert(
                laddr.clone(),
                Self::recv_from_owned(socket.clone(), max_recv_udp_payload_size),
            );

            sockets.insert(laddr, socket);
        }

        Ok(Self {
            sockets: Arc::new(sockets),
            max_recv_udp_payload_size,
            recv_map,
        })
    }

    async fn recv_from_owned(
        socket: Arc<UdpSocket>,
        max_recv_udp_payload_size: u16,
    ) -> Result<(Vec<u8>, SocketAddr)> {
        let mut buf = vec![0; max_recv_udp_payload_size as usize];

        let (read_size, from) = socket.recv_from(&mut buf).await?;

        buf.resize(read_size, 0);

        Ok((buf, from))
    }

    #[allow(unused)]
    /// Returns an iterator over the local binding addresses for this `UdpGroup`.
    pub fn local_addrs(&self) -> impl Iterator<Item = &SocketAddr> {
        self.sockets.keys()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the recv_buf and the path information.
    pub async fn recv_from(&self) -> Result<(Vec<u8>, PathInfo)> {
        while let Some((laddr, r)) = self.recv_map.as_ref().next().await {
            // As a whole, when one socket int this group calling `recv_from` failed,
            // the state of the group is undefined. therefore, we raise the error to
            // the caller.
            let (buf, raddr) = r?;

            let udp_socket = self.sockets.get(&laddr).unwrap().clone();

            // recv next packet.
            self.recv_map.insert(
                laddr,
                Self::recv_from_owned(udp_socket, self.max_recv_udp_payload_size),
            );

            return Ok((
                buf,
                PathInfo {
                    from: raddr,
                    to: laddr,
                },
            ));
        }

        Err(Error::new(ErrorKind::BrokenPipe, "UdpGroup broken"))
    }

    /// Sends data on the group with provided `path_info`.
    ///
    /// On success, returns the number of bytes written.
    pub async fn send_to<Buf: AsRef<[u8]>>(&self, buf: Buf, path_info: PathInfo) -> Result<usize> {
        if let Some(socket) = self.sockets.get(&path_info.from) {
            socket.send_to(buf.as_ref(), path_info.to).await
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                format!("Socket bound to {:?} is not in the group.", path_info.from),
            ))
        }
    }
}

pub trait QuicListenerBind {
    /// Create a new QuicListener instance with local addresses bound to `laddrs`.
    fn bind<A: ToSocketAddrs + Send>(
        laddrs: A,
        config: Config,
    ) -> impl Future<Output = Result<QuicListener>> + Send {
        async move {
            let laddrs = laddrs.to_socket_addrs()?.collect::<Vec<_>>();

            // max_recv_udp_payload_size must be greater than or equal to the maximum mtu size.
            // generally, the maximum mtu is 1,500 bytes.
            let udp_group = UdpGroup::bind(laddrs.as_slice(), MAX_MTU_SIZE as u16).await?;

            let laddrs = udp_group.local_addrs().cloned().collect::<Vec<_>>();

            let listener = QuicListener::new(laddrs.as_slice(), config)?;

            spawn_ok(listener_recv_loop(udp_group.clone(), listener.clone()));

            spawn_ok(listener_send_loop(udp_group, listener.clone()));

            Ok(listener)
        }
    }
}

async fn listener_send_loop(group: UdpGroup, listener: QuicListener) {
    if let Err(err) = listener_send_loop_priv(group, listener).await {
        log::error!(target: "QuicListener","stop send loop by error: {}",err);
    }
}

async fn listener_send_loop_priv(group: UdpGroup, listener: QuicListener) -> Result<()> {
    loop {
        let (buf, send_info) = listener.send().await?;

        log::trace!(
            "server_send: tx len={}, from={}, to={}",
            buf.len(),
            send_info.from,
            send_info.to
        );

        group
            .send_to(
                buf,
                PathInfo {
                    from: send_info.from,
                    to: send_info.to,
                },
            )
            .await?;
    }
}

async fn listener_recv_loop(group: UdpGroup, listener: QuicListener) {
    if let Err(err) = listener_recv_loop_priv(group, listener).await {
        log::error!(target: "QuicListener","stop recv loop by error: {}",err);
    }
}

async fn listener_recv_loop_priv(group: UdpGroup, listener: QuicListener) -> Result<()> {
    loop {
        log::trace!("server_recv");

        let (buf, path_info) = group.recv_from().await?;

        log::trace!(
            "server_recv: tx len={}, from={}, to={}",
            buf.len(),
            path_info.from,
            path_info.to
        );

        let (_, handshake) = listener
            .recv(
                buf,
                RecvInfo {
                    from: path_info.from,
                    to: path_info.to,
                },
            )
            .await?;

        if let Some(buf) = handshake {
            let send_info = PathInfo {
                from: path_info.to,
                to: path_info.from,
            };
            log::trace!(
                "server: tx handshake len={} from={}, to={}",
                buf.len(),
                send_info.from,
                send_info.to
            );
            // send handshake internal packet: version and retry, etc.
            group.send_to(buf, send_info).await?;
        }
    }
}

impl QuicListenerBind for QuicListener {}

pub trait QuicConnect {
    fn connect<L: ToSocketAddrs + Send, R: ToSocketAddrs + Send>(
        server_name: Option<&str>,
        laddrs: L,
        raddrs: R,
        config: &mut Config,
    ) -> impl Future<Output = Result<QuicConn>> + Send {
        async move {
            let laddrs = laddrs.to_socket_addrs()?.collect::<Vec<_>>();

            let raddrs = raddrs.to_socket_addrs()?.collect::<Vec<_>>();

            let mut last_error = Some(Error::new(
                ErrorKind::InvalidInput,
                "Can't connect to peer via local bind sockets",
            ));

            for raddr in raddrs {
                let laddr = if let Some(laddr) = laddrs
                    .iter()
                    .filter(|addr| raddr.is_ipv4() == addr.is_ipv4())
                    .choose(&mut thread_rng())
                {
                    laddr.clone()
                } else {
                    continue;
                };

                match Self::connect_on_path(server_name, laddr, raddr, config).await {
                    Ok(conn) => return Ok(conn),
                    Err(err) => last_error = Some(err),
                }
            }

            return Err(last_error.unwrap());
        }
    }

    fn connect_on_path(
        server_name: Option<&str>,
        laddr: SocketAddr,
        raddr: SocketAddr,
        config: &mut Config,
    ) -> impl Future<Output = Result<QuicConn>> + Send {
        async move {
            let udp_socket = UdpSocket::bind(laddr).await?;

            let laddr = udp_socket.local_addr()?;

            let mut scid = vec![0; quiche::MAX_CONN_ID_LEN];

            SystemRandom::new()
                .fill(&mut scid)
                .map_err(|err| Error::new(ErrorKind::Other, format!("{}", err)))?;

            let scid = quiche::ConnectionId::from_vec(scid);

            let conn = quiche::connect(server_name, &scid, laddr, raddr, config)
                .map_err(map_quic_error)?;

            let conn = QuicConnState::new(conn, 0, None);

            let mut buf = vec![0; MAX_MTU_SIZE];

            loop {
                let (send_size, send_info) = conn.send(&mut buf).await?;

                udp_socket.send_to(&buf[..send_size], send_info.to).await?;

                let (read_size, from) = if let Some(timeout_at) = conn.timeout_instant().await {
                    log::trace!(
                        "connect recv packet: scid={:?}, timeout={:?}",
                        conn.scid(),
                        timeout_at,
                    );
                    match udp_socket.recv_from(&mut buf).timeout_at(timeout_at).await {
                        Some(Ok((read_size, from))) => (read_size, from),
                        Some(Err(err)) => return Err(err),
                        None => {
                            log::warn!("connect recv packet, timeout: scid={:?}", conn.scid());
                            conn.on_timeout().await;
                            continue;
                        }
                    }
                } else {
                    log::trace!("connect recv packet: scid={:?}", conn.scid());
                    udp_socket.recv_from(&mut buf).await?
                };

                log::trace!("Connect: rx handshake len={}, from={}", read_size, from);

                conn.recv(&mut buf[..read_size], RecvInfo { from, to: laddr })
                    .await?;

                if conn.is_established().await {
                    break;
                }
            }

            spawn_ok(client_send_loop(udp_socket.clone(), conn.clone()));

            spawn_ok(client_recv_loop(udp_socket, conn.clone()));

            Ok(conn.into())
        }
    }
}

impl QuicConnect for QuicConn {}

async fn client_send_loop(udp_socket: UdpSocket, conn: QuicConnState) {
    if let Err(err) = client_send_loop_priv(udp_socket, &conn).await {
        log::error!(target: "QuicConn","stop recv loop by error: {}",err);
    }

    _ = conn.close();
}

async fn client_send_loop_priv(udp_socket: UdpSocket, conn: &QuicConnState) -> Result<()> {
    let mut buf = vec![0; MAX_MTU_SIZE];
    loop {
        let (send_size, send_info) = conn.send(&mut buf).await?;

        log::trace!(
            "client_send: tx len={}, from={}, to={}",
            send_size,
            send_info.from,
            send_info.to
        );

        udp_socket.send_to(&buf[..send_size], send_info.to).await?;
    }
}
async fn client_recv_loop(udp_socket: UdpSocket, conn: QuicConnState) {
    if let Err(err) = client_recv_loop_prev(udp_socket, &conn).await {
        log::error!(target: "QuicConn","stop recv loop by error: {}",err);
    }

    _ = conn.close();
}

async fn client_recv_loop_prev(udp_socket: UdpSocket, conn: &QuicConnState) -> Result<()> {
    let laddr = udp_socket.local_addr()?;
    let mut buf = vec![0; MAX_MTU_SIZE];
    loop {
        let (read_size, raddr) = udp_socket.recv_from(&mut buf).await?;

        log::trace!(
            "client_recv: tx len={}, from={}, to={}",
            read_size,
            raddr,
            laddr
        );

        conn.recv(
            &mut buf[..read_size],
            RecvInfo {
                from: raddr,
                to: laddr,
            },
        )
        .await?;
    }
}
