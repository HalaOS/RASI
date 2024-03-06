//! Utility to batch poll a set of [`udp sockets`](rasi::net::UdpSocket)
//!
//! UdpGroup internally uses `batching::Group` to drive the batch polling. [*Read more*](crate::future::batching::Group)
use std::{
    collections::HashMap,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use bytes::{Bytes, BytesMut};
use rasi::{
    futures::{Sink, Stream},
    net::UdpSocket,
    syscall::{global_network, Network},
};

/// A configuration for batch poll a set of [`udp socket`](rasi::net::UdpSocket)s
pub struct UdpGroup {
    #[allow(unused)]
    /// Inner socket map that mapping local_addr => socket.
    sockets: HashMap<SocketAddr, Arc<UdpSocket>>,
    /// The max buf length for batch reading.
    max_recv_buf_len: u16,
}

impl UdpGroup {
    /// Use global registered syscall interface [`Network`] to create a UDP socket group from the given address.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this socket. The
    /// port allocated can be queried via the [`local_addr`] method.
    ///
    /// [`local_addr`]: #method.local_addr
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { rasi::futures::executor::block_on(async {
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
    /// # fn main() -> std::io::Result<()> { rasi::futures::executor::block_on(async {
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
    /// # fn main() -> std::io::Result<()> { rasi::futures::executor::block_on(async {
    /// #
    /// use rasi_ext::net::udp_group::UdpGroup;
    ///
    /// let (sx,rx) = UdpGroup::bind("127.0.0.1:0").await?.split();
    /// #
    /// # Ok(()) }) }
    pub fn split(self) -> (Sender, Receiver) {
        (Sender {}, Receiver {})
    }
}

/// Udp data transfer metadata for the data sent to the peers
/// from the specified local address `from`` to the destination address `to`.
pub struct PathInfo {
    /// The specified local address.
    pub from: SocketAddr,
    /// The destination address for the data sent to the peer.
    pub to: SocketAddr,
}

/// Data is received from the peers via this [`UdpGroup`] receiver [`stream`](Stream).
pub struct Receiver {}

impl Stream for Receiver {
    type Item = io::Result<(BytesMut, PathInfo)>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}

/// Data is sent to the peers via this [`UdpGroup`] sender [`sink`](Sink).
pub struct Sender {}

#[allow(unused)]
impl Sink<(Bytes, Option<PathInfo>)> for Sender {
    type Error = io::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn start_send(
        self: std::pin::Pin<&mut Self>,
        item: (Bytes, Option<PathInfo>),
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }
}
