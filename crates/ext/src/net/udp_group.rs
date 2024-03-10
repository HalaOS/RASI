//! Utility to batch poll a set of [`udp sockets`](rasi::net::UdpSocket)
//!
//! UdpGroup internally uses `batching::Group` to drive the batch polling. [*Read more*](crate::future::batching::Group)
use std::{
    collections::HashMap,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
    task::Poll,
};

use bytes::BytesMut;
use rand::seq::IteratorRandom;
use rasi::{
    executor::spawn,
    net::UdpSocket,
    syscall::{global_network, Network},
};

use futures::{SinkExt, Stream, StreamExt};

use crate::utils::ReadBuf;

/// Udp data transfer metadata for the data sent to the peers
/// from the specified local address `from`` to the destination address `to`.
#[derive(Debug)]
pub struct PathInfo {
    /// The specified local address.
    pub from: SocketAddr,
    /// The destination address for the data sent to the peer.
    pub to: SocketAddr,
}

impl PathInfo {
    /// swap ***from*** field and ***to*** field of this object.
    pub fn reverse(self) -> Self {
        Self {
            from: self.to,
            to: self.from,
        }
    }
}

/// A configuration for batch poll a set of [`udp socket`](rasi::net::UdpSocket)s
pub struct UdpGroup {
    /// Inner socket map that mapping local_addr => socket.
    sockets: HashMap<SocketAddr, Arc<UdpSocket>>,
    /// The max buf length for batch reading.
    max_recv_buf_len: u16,
}

impl UdpGroup {
    /// Use global registered syscall interface [`Network`] to create a UDP socket group from the given address.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this socket. The
    /// port allocated can be queried via the [`local_addrs`](Self::local_addrs) method.
    ///
    /// [`local_addr`]: #method.local_addr
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi_ext::net::udp_group::UdpGroup;
    ///
    /// let socket = UdpGroup::bind("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn bind<A: ToSocketAddrs>(laddrs: A) -> io::Result<Self> {
        Self::bind_with(laddrs, global_network()).await
    }

    /// Use custom syscall interface [`Network`] to create a UDP socket group from the given address.
    /// [*Read more*](Self::bind)
    pub async fn bind_with<A: ToSocketAddrs>(
        laddrs: A,
        syscall: &'static dyn Network,
    ) -> io::Result<Self> {
        let laddrs = laddrs.to_socket_addrs()?.collect::<Vec<_>>();

        let mut sockets = HashMap::new();

        for laddr in laddrs {
            let socket = Arc::new(UdpSocket::bind_with([laddr].as_slice(), syscall).await?);

            // get the port allocated by OS.
            let laddr = socket.local_addr()?;

            sockets.insert(laddr, socket);
        }

        Ok(Self {
            sockets,
            max_recv_buf_len: 2048,
        })
    }

    /// Sets the capacity of batch reading buf to specific `len`.
    ///
    /// ***assert***: the specific `len` must > 0 && < 65535.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi_ext::net::udp_group::UdpGroup;
    ///
    /// let config = UdpGroup::bind("127.0.0.1:0").await?;
    /// let config = config.with_max_recv_buf_len(1024);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn with_max_recv_buf_len(mut self, len: u16) -> Self {
        assert!(len > 0, "sets max_recv_buf_len to zero");
        self.max_recv_buf_len = len;

        self
    }

    /// Helper method for splitting `UdpGroup` object into two halves.
    ///
    /// The two halves returned implement the [Sink] and [Stream] traits, respectively.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi_ext::net::udp_group::UdpGroup;
    ///
    /// let (sx,rx) = UdpGroup::bind("127.0.0.1:0").await?.split();
    /// #
    /// # Ok(()) }) }
    pub fn split(self) -> (Sender, Receiver) {
        let sockets = self.sockets.values().cloned().collect::<Vec<_>>();

        let (sender, receiver) = futures::channel::mpsc::channel(0);

        for socket in sockets {
            spawn(Self::recv_loop(
                socket,
                sender.clone(),
                self.max_recv_buf_len as usize,
            ));
        }

        (Sender::new(self.sockets), Receiver::new(receiver))
    }

    /// Returns the local addresses iterator that this udp group are bound to.
    ///
    /// This can be useful, for example, to identify when binding to port 0 which port was assigned
    /// by the OS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi_ext::net::udp_group::UdpGroup;
    ///
    /// let udp_group = UdpGroup::bind("127.0.0.1:0").await?;
    /// let laddrs = udp_group.local_addrs().collect::<Vec<_>>();
    /// #
    /// # Ok(()) }) }
    pub fn local_addrs(&self) -> impl Iterator<Item = &SocketAddr> {
        self.sockets.keys()
    }

    async fn recv_loop(
        socket: Arc<UdpSocket>,
        mut sender: futures::channel::mpsc::Sender<UdpGroupData>,
        max_recv_buf_len: usize,
    ) {
        let laddr = socket.local_addr().unwrap();

        loop {
            let mut read_buf = ReadBuf::with_capacity(max_recv_buf_len);

            match socket.recv_from(read_buf.chunk_mut()).await {
                Ok((read_size, raddr)) => {
                    let data = UdpGroupData {
                        result: Ok((read_buf.into_bytes_mut(Some(read_size)), raddr)),
                        to: laddr,
                    };

                    if sender.send(data).await.is_err() {
                        log::trace!("socket({:?}) in udp group, stop recv loop", laddr);
                    }
                }
                Err(err) => {
                    log::error!(
                        "socket({:?}) in udp group, shutdown with error: {}",
                        laddr,
                        err
                    );
                }
            }
        }
    }
}

struct UdpGroupData {
    result: io::Result<(BytesMut, SocketAddr)>,
    /// the receiver socket's local address.
    to: SocketAddr,
}

/// Data is received from the peers via this [`UdpGroup`] receiver [`stream`](Stream).
pub struct Receiver {
    inner: futures::channel::mpsc::Receiver<UdpGroupData>,
}

impl Receiver {
    fn new(inner: futures::channel::mpsc::Receiver<UdpGroupData>) -> Self {
        Self { inner }
    }
}

impl Stream for Receiver {
    type Item = io::Result<(BytesMut, PathInfo)>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(udp_group_data)) => {
                Poll::Ready(Some(udp_group_data.result.map(|(buf, raddr)| {
                    (
                        buf,
                        PathInfo {
                            from: raddr,
                            to: udp_group_data.to,
                        },
                    )
                })))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Data is sent to the peers via this [`UdpGroup`] sender
pub struct Sender {
    sockets: Arc<HashMap<SocketAddr, Arc<UdpSocket>>>,
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Self {
            sockets: self.sockets.clone(),
        }
    }
}

impl Sender {
    fn new(sockets: HashMap<SocketAddr, Arc<UdpSocket>>) -> Self {
        Self {
            sockets: Arc::new(sockets),
        }
    }

    /// Sends data on the random socket in this group to the given address.
    ///
    /// On success, returns the number of bytes written.
    pub async fn send_to(&self, buf: &[u8], raddr: SocketAddr) -> io::Result<usize> {
        let socket = self
            .sockets
            .values()
            .choose(&mut rand::thread_rng())
            .unwrap()
            .clone();

        socket.send_to(buf, raddr).await
    }

    /// Sends data on the [`PathInfo`] to the given address.
    ///
    /// On success, returns the number of bytes written.
    pub async fn send_to_on_path(&self, buf: &[u8], path_info: PathInfo) -> io::Result<usize> {
        if let Some(socket) = self.sockets.get(&path_info.from) {
            socket.send_to(buf, path_info.to).await
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Socket bound to {:?} is not in the group.", path_info.from),
            ))
        }
    }
}

#[cfg(test)]
mod tests {

    use bytes::Bytes;
    use futures::TryStreamExt;
    use rasi_default::{executor::register_futures_executor, net::MioNetwork};

    use super::*;

    use std::{net::SocketAddr, sync::OnceLock};

    static INIT: OnceLock<Box<dyn rasi::syscall::Network>> = OnceLock::new();

    fn get_syscall() -> &'static dyn rasi::syscall::Network {
        INIT.get_or_init(|| {
            register_futures_executor(10).unwrap();
            Box::new(MioNetwork::default())
        })
        .as_ref()
    }

    #[futures_test::test]
    async fn test_udp_group_echo() {
        let syscall = get_syscall();

        let addrs: Vec<SocketAddr> = ["127.0.0.1:0".parse().unwrap()].repeat(4);
        let (client_sender, mut client_receiver) = UdpGroup::bind_with(addrs.as_slice(), syscall)
            .await
            .unwrap()
            .split();

        let server = UdpGroup::bind_with(addrs.as_slice(), syscall)
            .await
            .unwrap();

        let raddrs = server.local_addrs().cloned().collect::<Vec<_>>();

        let (server_sender, mut server_receiver) = server.split();

        let random_raddr = raddrs
            .iter()
            .choose(&mut rand::thread_rng())
            .cloned()
            .unwrap();

        client_sender
            .send_to(b"hello world", random_raddr)
            .await
            .unwrap();

        let (buf, path_info) = server_receiver.try_next().await.unwrap().unwrap();

        let buf = buf.freeze();

        assert_eq!(buf, Bytes::from_static(b"hello world"));

        server_sender
            .send_to_on_path(b"hello world", path_info.reverse())
            .await
            .unwrap();

        let (buf, _) = client_receiver.try_next().await.unwrap().unwrap();

        let buf = buf.freeze();

        assert_eq!(buf, Bytes::from_static(b"hello world"));
    }
}
