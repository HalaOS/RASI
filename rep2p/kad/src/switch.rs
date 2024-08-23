//! This module provide a rep2p compatibable kad implementation.

use std::{collections::HashMap, fmt::Debug, num::NonZeroUsize, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    SinkExt, StreamExt,
};
use identity::PeerId;
use rasi::{task::spawn_ok, timer::TimeoutExt};
use rep2p::{
    book::PeerInfo,
    multiaddr::Multiaddr,
    serve::{
        syscall::{DriverProtocol, DriverProtocolHandler},
        ProtocolHandler,
    },
    transport::Stream,
    Switch,
};

use crate::{
    errors::{Error, Result}, kbucket::KBucketKey, route_table::{syscall::DriverKadRouteTable, KBucketRouteTable, KadRouteTable}, routing::{FindNode, Query, Router}, rpc::KadRpc, Key
};

/// protocol name of libp2p kad.
pub const PROTOCOL_IPFS_KAD: &str = "/ipfs/kad/1.0.0";
pub const PROTOCOL_IPFS_LAN_KAD: &str = "/ipfs/lan/kad/1.0.0";

/// A protocol stack than support libp2p kad network.
#[derive(Clone)]
pub struct KadSwitch {
    /// The maximum length of kad rpc packet.
    pub(crate) max_packet_len: usize,
    /// timeout intervals of kad RPCs.
    pub(crate) timeout: Duration,
    /// The limits of concurrency of node and value lookups.
    concurrency: NonZeroUsize,
    /// underlying libp2p switch.
    pub(crate) switch: Switch,
    /// the kad route table implementation.
    pub(crate) route_table: Arc<KadRouteTable>,
}

impl KadSwitch {}

impl KadSwitch {
    /// Create a new kad switch instance.
    pub fn new<R>(switch: &Switch, route_table: R) -> Self
    where
        R: DriverKadRouteTable + 'static,
    {
        Self {
            max_packet_len: 1024 * 1024,
            timeout: Duration::from_secs(10),
            concurrency: NonZeroUsize::new(10).unwrap(),
            switch: switch.clone(),
            route_table: Arc::new(KadRouteTable::from(route_table)),
        }
    }

    /// Uses the seeds to init this kad node's route table.
    pub async fn with_seeds<S, E>(self, seeds: S) -> Result<Self>
    where
        S: IntoIterator,
        S::Item: TryInto<Multiaddr, Error = E>,
        E: Debug,
    {
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
            self.route_table.insert(id.clone()).await?;

            let peer_info = PeerInfo {
                id: id.clone(),
                addrs,
                ..Default::default()
            };

            self.switch.update_peer_info(peer_info).await?;
        }

        Ok(self)
    }

    /// Run the recursive query process.
    pub async fn route<Q>(&self, query: Q) -> Result<(Q, Vec<PeerId>)>
    where
        Q: Query,
    {
        Router::new(self, query).route(self.concurrency).await
    }

    /// Invoke a kad `FIND_NODE` process.
    pub async fn find_node(&self, peer_id: &PeerId) -> Result<Option<PeerInfo>> {
        let (find_node, closest) = self
            .route(FindNode::new(
                &self.switch,
                self.max_packet_len,
                peer_id,
                self.timeout,
            ))
            .await?;

        let target_key = Key::from(peer_id);

        let closest = closest
            .iter()
            .map(|id| Key::from(id).distance(&target_key).to_string())
            .collect::<Vec<_>>();

        log::trace!("find_node id={}, closest={:?}", peer_id, closest);

        Ok(find_node.into_peer_info().await)
    }

    pub async fn put_value<K, V>(&self, key: K, value: V) -> Result<usize>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        let (_, closest) = self
            .route(FindNode::new(
                &self.switch,
                self.max_packet_len,
                key,
                self.timeout,
            ))
            .await?;

        let (sender, mut receiver) = channel(closest.len());

        let mut tasks = 0usize;

        for peer_id in closest {
            let key = key.to_vec();
            let value = value.to_vec();

            let this = self.clone();
            let mut sender = sender.clone();

            spawn_ok(async move {
                log::trace!("PUT_VALUE start: target={}", peer_id);
                if let Err(err) = this.put_value_prv(&peer_id, key, value).await {
                    log::error!("PUT_VALUE error: target={}, err={}", peer_id, err);
                    _ = sender.send(false).await;
                } else {
                    log::error!("PUT_VALUE success: target={}, success", peer_id);
                    _ = sender.send(true).await;
                }
            });

            tasks += 1;
        }

        let mut succ = 0usize;

        for i in 0..tasks {
            if let Some(true) = receiver.next().await {
                succ += 1;
            }
            log::trace!("PUT_VALUE: total={}, current={}, succ={}", tasks, i, succ);
        }

        Ok(succ)
    }

    async fn put_value_prv(&self, peer_id: &PeerId, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        if let Some(stream) = connect(self.switch.clone(), peer_id, self.timeout).await {
            log::trace!("PUT_VALUE open stream success: target={}.", peer_id);
            stream
                .kad_put_value(key, value, self.max_packet_len)
                .timeout(self.timeout)
                .await
                .ok_or(Error::Timeout)??;

            Ok(())
        } else {
            Err(Error::PutValue(peer_id.clone()))
        }
    }
}

pub(crate) async fn connect(switch: Switch, peer_id: &PeerId, timeout: Duration) -> Option<Stream> {
    match switch
        .open(peer_id, [PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD])
        .timeout(timeout)
        .await
    {
        Some(Ok((stream, _))) => return Some(stream),
        Some(Err(err)) => {
            log::error!("connect to peer, id={}, err={}", peer_id, err);
        }
        _ => {
            log::error!(
                "connect to peer, id={}, err='timeout({:?})'",
                peer_id,
                timeout
            );
        }
    }

    None
}

/// A [`ServeMux`](rep2p::serve::ServeMux) compatibable kad protocol implementation.
pub struct KadProtocol(Vec<Multiaddr>, Sender<KadSwitch>);

impl KadProtocol {
    pub fn new<S, E>(seeds: S) -> Result<(Self, Receiver<KadSwitch>)>
    where
        S: IntoIterator,
        S::Item: TryInto<Multiaddr, Error = E>,
        E: Debug,
    {
        let seeds = seeds
            .into_iter()
            .map(|item| item.try_into())
            .collect::<std::result::Result<Vec<Multiaddr>, E>>()
            .map_err(|err| Error::Other(format!("{:?}", err)))?;

        let (sender, receiver) = channel(0);

        Ok((Self(seeds, sender), receiver))
    }
}

#[async_trait]
impl DriverProtocol for KadProtocol {
    /// Returns protocol display name.
    fn protos(&self) -> &'static [&'static str] {
        &[PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD]
    }

    async fn create(&self, switch: &Switch) -> std::io::Result<ProtocolHandler> {
        let switch = KadSwitch::new(&switch, KBucketRouteTable::new(switch.local_id()))
            .with_seeds(self.0.clone())
            .await?;

        // ignore the send result.
        _ = self.1.clone().send(switch.clone()).await;

        Ok(switch.into())
    }
}

#[async_trait]
impl DriverProtocolHandler for KadSwitch {
    /// Handle a new incoming stream.
    async fn dispatch(&self, _negotiated: &str, _stream: Stream) -> std::io::Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Once};

    use rasi_mio::{net::register_mio_network, timer::register_mio_timer};
    use rep2p::{multiaddr::multihash::Multihash, Switch};
    use rep2p_quic::QuicTransport;
    use rep2p_tcp::TcpTransport;
    use sha2::Digest;

    use crate::route_table::KBucketRouteTable;

    use super::*;

    async fn init() -> Switch {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            _ = pretty_env_logger::try_init_timed();

            register_mio_network();
            register_mio_timer();
        });

        let switch = Switch::new("kad-test")
            .transport(QuicTransport::default())
            .transport(TcpTransport)
            // .protos([PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD])
            .create()
            .await
            .unwrap();

        log::trace!("create switch: {}", switch.local_id());

        switch
    }

    #[futures_test::test]
    async fn find_node() {
        let switch = init().await;

        let kad = KadSwitch::new(&switch, KBucketRouteTable::new(switch.local_id()))
            .with_seeds([
                "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
                "/ip4/104.131.131.82/udp/4001/quic-v1/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ"
            ])
            .await
            .unwrap();

        let peer_id = PeerId::random();

        let peer_info = kad.find_node(&peer_id).await.unwrap();

        log::info!("find_node: {}, {:?}", peer_id, peer_info);
    }

    #[futures_test::test]
    async fn find_node_1() {
        let switch = init().await;

        let kad = KadSwitch::new(&switch, KBucketRouteTable::new(switch.local_id()))
            .with_seeds([
                "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
                "/ip4/104.131.131.82/udp/4001/quic-v1/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ"
            ])
            .await
            .unwrap();

        let peer_id =
            PeerId::from_str("12D3KooWKp6C6U2FRBghfjpbpYzU2XgBu9KpTKjHj8fRtsNtXjdZ").unwrap();

        let peer_info = kad.find_node(&peer_id).await.unwrap();

        log::info!("find_node: {}, {:?}", peer_id, peer_info);
    }

    #[futures_test::test]
    async fn put_value() {
        let switch = init().await;

        let kad = KadSwitch::new(&switch, KBucketRouteTable::new(switch.local_id()))
            .with_seeds([
                "/ip4/104.131.131.82/udp/4001/quic-v1/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
                "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
                
            ])
            .await
            .unwrap();

        let peer_id = PeerId::random();

        let put_nodes = kad.put_value(peer_id.to_bytes(), "world").await.unwrap();

        log::info!("put value, peer_id={}, nodes={}", peer_id, put_nodes);
    }


    #[futures_test::test]
    async fn put_value_1() {
        let switch = init().await;

        let kad = KadSwitch::new(&switch, KBucketRouteTable::new(switch.local_id()))
            .with_seeds([
                "/ip4/127.0.0.1/udp/4001/quic-v1/p2p/12D3KooWLjoYKVxbGGwLwaD4WHWM9YiDpruCYAoFBywJu3CJppyB",
                "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWLjoYKVxbGGwLwaD4WHWM9YiDpruCYAoFBywJu3CJppyB",
                
            ])
            .await
            .unwrap();

        let keypair = identity::Keypair::generate_ed25519();



        let peer_id: PeerId = "12D3KooWLjoYKVxbGGwLwaD4WHWM9YiDpruCYAoFBywJu3CJppyB".parse().unwrap();

        let (stream,_) = kad.switch.open(&peer_id, [PROTOCOL_IPFS_KAD,PROTOCOL_IPFS_LAN_KAD]).await.unwrap();

        log::trace!("Begin put value");

        let mut hasher = sha2::Sha256::new();

        hasher.update(&keypair.public().encode_protobuf());

        let hash: Multihash<64> = Multihash::wrap(0x12, &hasher.finalize()).unwrap();

        stream.kad_put_value(format!("/test/{}",bs58::encode(hash.to_bytes()).into_string()), "world", 1024*1024).await.unwrap();
    }
}
