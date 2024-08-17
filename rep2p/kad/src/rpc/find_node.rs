use std::collections::HashSet;

use identity::PeerId;
use rep2p::multiaddr::Multiaddr;

use crate::{
    errors::Result,
    kbucket::KBucketKey,
    primitives::{Key, PeerInfo},
    KadSwitch, PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD,
};

use super::KadRpc;

/// The context of find_node process.
pub(crate) struct FindNode<'a> {
    state: &'a KadSwitch,
    peer_id: &'a PeerId,
    pq: HashSet<PeerId>,
    pn: Vec<PeerInfo>,
    key: Key,
}

impl<'a> FindNode<'a> {
    pub(crate) async fn new(state: &'a KadSwitch, peer_id: &'a PeerId) -> Self {
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

    pub(crate) async fn invoke(mut self) -> Result<Option<PeerInfo>> {
        for peer_info in self.state.route_table.closest(self.peer_id).await? {
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
                                self.state.route_table.insert(peer_info.clone()).await?;

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
            .connect(raddr, [PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD])
            .await?;

        stream.find_node(self.peer_id, 4096 * 1024).await
    }
}
