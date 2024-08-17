//! This module provide a rep2p compatibable kad implementation.

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use identity::PeerId;
use rep2p::{multiaddr::Multiaddr, Switch};

use crate::{
    errors::{Error, Result},
    primitives::PeerInfo,
    route_table::{syscall::DriverKadRouteTable, KadRouteTable},
    rpc::find_node::FindNode,
};

/// protocol name of libp2p kad.
pub const PROTOCOL_IPFS_KAD: &str = "/ipfs/kad/1.0.0";
pub const PROTOCOL_IPFS_LAN_KAD: &str = "/ipfs/lan/kad/1.0.0";

/// A protocol stack than support libp2p kad network.
#[derive(Clone)]
pub struct KadSwitch {
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
            let peer_info = PeerInfo {
                id: id.clone(),
                addrs,
                conn_type: Default::default(),
            };

            self.route_table.insert(peer_info).await?;
        }

        Ok(self)
    }

    /// Invoke a kad `FIND_NODE` process.
    pub async fn find_node(&self, peer_id: &PeerId) -> Result<Option<PeerInfo>> {
        let find_node = FindNode::new(self, peer_id).await;

        find_node.invoke().await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use rasi_mio::{net::register_mio_network, timer::register_mio_timer};
    use rep2p::Switch;
    use rep2p_quic::QuicTransport;
    use rep2p_tcp::TcpTransport;

    use crate::route_table::KBucketRouteTable;

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

        let kad = KadSwitch::new(&switch, KBucketRouteTable::new(switch.local_id()))
            .with_seeds([
                "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
            ])
            .await
            .unwrap();

        let peer_id = PeerId::random();

        kad.find_node(&peer_id).await.unwrap();
    }
}
