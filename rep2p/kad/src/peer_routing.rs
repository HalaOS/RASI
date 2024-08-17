use std::collections::HashSet;

use identity::PeerId;

use crate::{primitives::PeerInfo, KadSwitch};

/// The context of peer routing algorithm.
pub(crate) struct PeerRouting<'a> {
    switch: &'a KadSwitch,
    peer_id: &'a PeerId,
    /// The hashset of queried peer id.
    queried: HashSet<PeerId>,
    /// the set of next query candidates sorted by distance from `peer_id` in ascending order.
    /// At initialization `candidates` is seeded with the k peers from our routing table we know are closest to `peer_id`
    candidates: Vec<PeerInfo>,
}

impl<'a> PeerRouting<'a> {
    pub async fn new(switch: &'a KadSwitch, peer_id: &'a PeerId) -> Self {
        Self {
            switch,
            peer_id,
            queried: Default::default(),
            candidates: Default::default(),
        }
    }
}
