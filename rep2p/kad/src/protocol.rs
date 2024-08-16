//! This module provide a rep2p compatibable kad implementation.

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    future::Future,
    sync::Arc,
};

use futures::{lock::Mutex, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use identity::PeerId;
use protobuf::Message;
use rep2p::{
    multiaddr::{self, Multiaddr},
    Switch,
};

use crate::{
    kbucket::KBucketKey,
    primitives::{KBucketTable, Key, PeerInfo},
    proto::rpc::{self},
};

/// protocol name of libp2p kad.
pub const PROTOCOL_IPFS_KAD: &str = "/ipfs/kad/1.0.0";
pub const PROTOCOL_IPFS_LAN_KAD: &str = "/ipfs/lan/kad/1.0.0";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Multiaddr without p2p node, {0}")]
    InvalidSeedMultAddr(Multiaddr),

    #[error(transparent)]
    SwitchError(#[from] rep2p::Error),

    #[error(transparent)]
    IoErr(#[from] std::io::Error),

    #[error(transparent)]
    ProtocolError(#[from] protobuf::Error),

    #[error(transparent)]
    ReadError(#[from] unsigned_varint::io::ReadError),

    #[error("The rpc response length is greater than {0}")]
    ResponeLength(usize),

    #[error("Invalid find_node response type: {0}")]
    InvalidFindNodeResponse(String),

    #[error(transparent)]
    ParseError(#[from] identity::ParseError),

    #[error(transparent)]
    MultiaddrError(#[from] multiaddr::Error),
}

/// Result type returns by this module functionss.
pub type Result<T> = std::result::Result<T, Error>;

/// An extension to add kad rpc functions to [`AsyncWrite`]
pub trait KadRpc: AsyncWrite + AsyncRead + Unpin {
    fn find_node(
        mut self,
        peer_id: &PeerId,
        max_recv_len: usize,
    ) -> impl Future<Output = Result<Vec<PeerInfo>>>
    where
        Self: Sized,
    {
        let mut message = rpc::Message::new();

        message.type_ = rpc::message::MessageType::FIND_NODE.into();
        message.key = peer_id.to_bytes();

        async move {
            let buf = message.write_to_bytes()?;

            let mut payload_len = unsigned_varint::encode::usize_buffer();

            self.write_all(unsigned_varint::encode::usize(buf.len(), &mut payload_len))
                .await?;

            self.write_all(buf.as_slice()).await?;

            let body_len = unsigned_varint::aio::read_usize(&mut self).await?;

            if body_len > max_recv_len {
                return Err(Error::ResponeLength(max_recv_len));
            }

            let mut buf = vec![0u8; body_len];

            self.read_exact(&mut buf).await?;

            let message = rpc::Message::parse_from_bytes(&buf)?;

            if message.type_ != rpc::message::MessageType::FIND_NODE.into() {
                return Err(Error::InvalidFindNodeResponse(format!(
                    "{:?}",
                    message.type_
                )));
            }

            let mut peers = vec![];

            for peer in message.closerPeers {
                let mut addrs = vec![];

                for addr in peer.addrs {
                    addrs.push(Multiaddr::try_from(addr)?);
                }

                peers.push(PeerInfo {
                    id: PeerId::from_bytes(&peer.id)?,
                    addrs,
                    conn_type: peer.connection.enum_value_or_default(),
                });
            }

            Ok(peers)
        }
    }
}

impl<T> KadRpc for T where T: AsyncWrite + AsyncRead + Unpin {}

/// The context of find_node process.
struct FindNode<'a> {
    state: &'a KadState,
    peer_id: &'a PeerId,
    pq: HashSet<PeerId>,
    pn: Vec<PeerInfo>,
    key: Key,
}

impl<'a> FindNode<'a> {
    async fn new(state: &'a KadState, peer_id: &'a PeerId) -> Self {
        let key = Key::from(peer_id);

        let mut this = Self {
            state,
            peer_id,
            pq: HashSet::new(),
            pn: vec![],
            key,
        };

        this.sort_pn();

        this
    }

    fn sort_pn(&mut self) {
        self.pn.sort_by(|x, y| {
            let distance_y = Key::from(&y.id).distance(&self.key);

            let distance_x = Key::from(&x.id).distance(&self.key);

            distance_y.cmp(&distance_x)
        });
    }

    async fn invoke(mut self) -> Result<Option<PeerInfo>> {
        for (_, peer_info) in self.state.k_bucket_table.lock().await.closest_k(&self.key) {
            // find target in local k_bucket.
            if peer_info.id == *self.peer_id {
                log::info!(
                    "find_node: id={}, raddrs={:?}",
                    self.peer_id,
                    peer_info.addrs
                );

                return Ok(Some(peer_info.clone()));
            }

            self.pn.push(peer_info.clone());
        }

        self.sort_pn();

        while let Some(peer_info) = self.pn.pop() {
            log::trace!(
                "query: id={}, distance={}, pn={}",
                peer_info.id,
                Key::from(&peer_info.id).distance(&self.key),
                self.pn.len(),
            );

            self.pq.insert(peer_info.id);

            for raddr in peer_info.addrs {
                log::trace!("try query: id={}, raddr={}", peer_info.id, raddr);

                match self.find_node(&raddr).await {
                    Ok(peer_infos) => {
                        log::trace!("query success: id={}, raddr={}", peer_info.id, raddr);

                        for peer_info in peer_infos {
                            if !self.pq.contains(&peer_info.id) {
                                // update local k-buckets.
                                self.state
                                    .k_bucket_table
                                    .lock()
                                    .await
                                    .insert(Key::from(&peer_info.id), peer_info.clone());

                                // if this is target peer, return immediately.
                                if peer_info.id == *self.peer_id {
                                    log::info!(
                                        "find_node: id={}, raddrs={:?}",
                                        self.peer_id,
                                        peer_info.addrs
                                    );
                                    return Ok(Some(peer_info));
                                }

                                log::trace!(
                                    "pn add: id={}, raddrs={:?}",
                                    peer_info.id,
                                    peer_info.addrs
                                );

                                self.pn.push(peer_info);
                            }
                        }

                        self.sort_pn();
                        break;
                    }
                    Err(err) => {
                        log::error!("query: id={}, raddr={}, err={}", peer_info.id, raddr, err);
                        continue;
                    }
                }
            }
        }

        log::warn!("find_node: id={}, not found.", self.peer_id,);

        Ok(None)
    }

    async fn find_node(&self, raddr: &Multiaddr) -> Result<Vec<PeerInfo>> {
        let (stream, _) = self
            .state
            .switch
            .connect_to(&raddr, [PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD])
            .await?;

        stream.find_node(self.peer_id, 4096 * 1024).await
    }
}

/// This is a Kademlia node, and state machine.
#[allow(unused)]
pub struct KadState {
    switch: Switch,
    // the route table.
    k_bucket_table: Arc<Mutex<KBucketTable>>,
}

impl KadState {
    /// Create a new kademlia state machine with [`Switch`] and bootstrap `seeds`.
    ///
    /// On success, this function start the bootstrap process internal.
    pub async fn new<S, E>(switch: &Switch, seeds: S) -> Result<Self>
    where
        S: IntoIterator,
        S::Item: TryInto<Multiaddr, Error = E>,
        E: Debug,
    {
        let mut k_bucket_table = KBucketTable::new(Key::from(switch.local_id()));

        log::info!("start kademlia node bootstrap process..");

        let mut peer_addrs = HashMap::<PeerId, Vec<Multiaddr>>::new();

        for raddr in seeds.into_iter() {
            let raddr = raddr.try_into().unwrap();

            match raddr
                .clone()
                .pop()
                .ok_or_else(|| Error::InvalidSeedMultAddr(raddr.clone()))?
            {
                rep2p::multiaddr::Protocol::P2p(id) => {
                    if let Some(addrs) = peer_addrs.get_mut(&id) {
                        addrs.push(raddr);
                    } else {
                        peer_addrs.insert(id, vec![raddr]);
                    }
                }
                _ => {
                    return Err(Error::InvalidSeedMultAddr(raddr.clone()));
                }
            }
        }

        for (id, addrs) in peer_addrs {
            let peer_info = PeerInfo {
                id: id.clone(),
                addrs,
                conn_type: Default::default(),
            };

            k_bucket_table.insert(id, peer_info);
        }

        Ok(Self {
            switch: switch.clone(),
            k_bucket_table: Arc::new(Mutex::new(k_bucket_table)),
        })
    }

    pub async fn find_node(&self, peer_id: &PeerId) -> Result<Option<PeerInfo>> {
        let find_node = FindNode::new(self, peer_id).await;

        find_node.invoke().await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use rasi_mio::{net::register_mio_network, timer::register_mio_timer};
    use rep2p::{multiaddr::Multiaddr, Switch};
    use rep2p_quic::QuicTransport;
    use rep2p_tcp::TcpTransport;

    use super::*;

    async fn init() -> Switch {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            pretty_env_logger::init();

            register_mio_network();
            register_mio_timer();
        });

        let switch = Switch::new("kad-test")
            .transport(QuicTransport)
            .transport(TcpTransport)
            .protos([PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD])
            .create()
            .await
            .unwrap();

        log::trace!("create switch: {}", switch.local_id());

        switch
    }

    #[futures_test::test]
    async fn find_node() {
        let switch = init().await;

        let raddr: Multiaddr =
            "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ"
                .parse()
                .unwrap();

        let kad = KadState::new(&switch, [raddr]).await.unwrap();

        let peer_id = PeerId::random();

        kad.find_node(&peer_id).await.unwrap();
    }
}
