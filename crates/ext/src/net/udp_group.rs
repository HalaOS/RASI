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

use bytes::{Bytes, BytesMut};
use rand::seq::IteratorRandom;
use rasi::{
    net::UdpSocket,
    syscall::{global_network, Network},
};

use futures::{future::BoxFuture, FutureExt, Sink, Stream, StreamExt};

use crate::{future::batching, utils::ReadBuf};

/// Udp data transfer metadata for the data sent to the peers
/// from the specified local address `from`` to the destination address `to`.
#[derive(Debug)]
pub struct PathInfo {
    /// The specified local address.
    pub from: SocketAddr,
    /// The destination address for the data sent to the peer.
    pub to: SocketAddr,
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

        let (group, ready) = batching::Group::new();

        (
            Sender::new(group.clone(), self.sockets),
            Receiver::new(group, ready, &sockets, self.max_recv_buf_len),
        )
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
}

struct RecvFrom {
    group: batching::Group<(RecvFrom, io::Result<(BytesMut, PathInfo)>)>,
    socket: Arc<UdpSocket>,
    max_recv_buf_len: u16,
}

/// Data is received from the peers via this [`UdpGroup`] receiver [`stream`](Stream).
pub struct Receiver {
    /// Stream of batch poll result.
    ready: batching::Ready<(RecvFrom, io::Result<(BytesMut, PathInfo)>)>,
}

impl Receiver {
    fn new(
        group: batching::Group<(RecvFrom, io::Result<(BytesMut, PathInfo)>)>,
        ready: batching::Ready<(RecvFrom, io::Result<(BytesMut, PathInfo)>)>,
        sockets: &[Arc<UdpSocket>],
        max_recv_buf_len: u16,
    ) -> Self {
        for socket in sockets {
            group.join(Self::recv_from(RecvFrom {
                group: group.clone(),
                socket: socket.clone(),
                max_recv_buf_len,
            }));
        }

        Self { ready }
    }

    async fn recv_from(cx: RecvFrom) -> (RecvFrom, io::Result<(BytesMut, PathInfo)>) {
        let r = Self::recv_from_impl(cx.socket.clone(), cx.max_recv_buf_len).await;

        (cx, r)
    }

    async fn recv_from_impl(
        socket: Arc<UdpSocket>,
        max_recv_buf_len: u16,
    ) -> io::Result<(BytesMut, PathInfo)> {
        let mut buf = ReadBuf::with_capacity(max_recv_buf_len as usize);

        let (read_size, raddr) = socket.recv_from(buf.chunk_mut()).await?;

        let buf = buf.into_bytes_mut(Some(read_size));

        Ok((
            buf,
            PathInfo {
                from: raddr,
                to: socket.local_addr()?,
            },
        ))
    }
}

impl Stream for Receiver {
    type Item = io::Result<(BytesMut, PathInfo)>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.ready.poll_next_unpin(cx) {
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some((cx, r))) => {
                let group = cx.group.clone();

                group.join(Self::recv_from(cx));

                Poll::Ready(Some(r))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Data is sent to the peers via this [`UdpGroup`] sender [`sink`](Sink).
pub struct Sender {
    group: batching::Group<(RecvFrom, io::Result<(BytesMut, PathInfo)>)>,
    sockets: Arc<HashMap<SocketAddr, Arc<UdpSocket>>>,
    fut: Option<BoxFuture<'static, io::Result<usize>>>,
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Self {
            group: self.group.clone(),
            sockets: self.sockets.clone(),
            fut: None,
        }
    }
}

impl Sender {
    fn new(
        group: batching::Group<(RecvFrom, io::Result<(BytesMut, PathInfo)>)>,
        sockets: HashMap<SocketAddr, Arc<UdpSocket>>,
    ) -> Self {
        Self {
            group,
            sockets: Arc::new(sockets),
            fut: None,
        }
    }
}

impl Sink<(Bytes, Option<SocketAddr>, SocketAddr)> for Sender {
    type Error = io::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        if self.fut.is_none() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn start_send(
        mut self: std::pin::Pin<&mut Self>,
        item: (Bytes, Option<SocketAddr>, SocketAddr),
    ) -> Result<(), Self::Error> {
        let socket = if let Some(from) = item.1 {
            self.sockets
                .get(&from)
                .map(Clone::clone)
                .ok_or(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("UdpSocket not found, local_addr={:?}", from),
                ))?
        } else {
            // randomly selects one socket to send data.
            self.sockets
                .values()
                .choose(&mut rand::thread_rng())
                .unwrap()
                .clone()
        };

        self.fut = Some(Box::pin(
            async move { socket.send_to(&item.0, item.2).await },
        ));

        Ok(())
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        if let Some(mut fut) = self.fut.take() {
            match fut.poll_unpin(cx) {
                Poll::Ready(Ok(_)) => return Poll::Ready(Ok(())),
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Pending => {}
            }
        }

        Poll::Pending
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.group.close();

        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {

    use futures::{SinkExt, TryStreamExt};
    use rasi_default::net::MioNetwork;

    use super::*;

    use std::{net::SocketAddr, sync::OnceLock};

    static INIT: OnceLock<Box<dyn rasi::syscall::Network>> = OnceLock::new();

    fn get_syscall() -> &'static dyn rasi::syscall::Network {
        INIT.get_or_init(|| Box::new(MioNetwork::default()))
            .as_ref()
    }

    #[futures_test::test]
    async fn test_udp_group_echo() {
        let syscall = get_syscall();

        let addrs: Vec<SocketAddr> = ["127.0.0.1:0".parse().unwrap()].repeat(4);
        let (mut client_sender, mut client_receiver) =
            UdpGroup::bind_with(addrs.as_slice(), syscall)
                .await
                .unwrap()
                .split();

        let server = UdpGroup::bind_with(addrs.as_slice(), syscall)
            .await
            .unwrap();

        let raddrs = server.local_addrs().cloned().collect::<Vec<_>>();

        let (mut server_sender, mut server_receiver) = server.split();

        let random_raddr = raddrs
            .iter()
            .choose(&mut rand::thread_rng())
            .cloned()
            .unwrap();

        client_sender
            .send((Bytes::from_static(b"hello world"), None, random_raddr))
            .await
            .unwrap();

        let (buf, path_info) = server_receiver.try_next().await.unwrap().unwrap();

        let buf = buf.freeze();

        assert_eq!(buf, Bytes::from_static(b"hello world"));

        server_sender
            .send((
                Bytes::from_static(b"hello world"),
                Some(path_info.to),
                path_info.from,
            ))
            .await
            .unwrap();

        let (buf, _) = client_receiver.try_next().await.unwrap().unwrap();

        let buf = buf.freeze();

        assert_eq!(buf, Bytes::from_static(b"hello world"));
    }
}
