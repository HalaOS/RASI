//! This module provide a rep2p compatibable kad implementation.

use rep2p::{multiaddr::Multiaddr, Result, Switch};

use crate::primitives::KBucketTable;

/// protocol name of libp2p kad.
pub const PROTOCOL_IPFS_KAD: &str = "/ipfs/kad/1.0.0";

/// This is a Kademlia node, and state machine.
pub struct KademliaState {
    // the route table.
    k_bucket_table: KBucketTable,
}

impl KademliaState {
    /// Start kademlia node bootstrap process,
    /// add self into the kademlia network by sending `FIND_NODE` requests to the seed nodes.
    ///
    /// On succeed, returns the node instance.
    pub async fn bootstrap<S>(switch: &Switch, seeds: S) -> Result<Self>
    where
        S: IntoIterator,
        S::Item: AsRef<Multiaddr>,
    {
        log::info!("start kademlia node bootstrap process..");

        for raddr in seeds.into_iter() {
            let raddr = raddr.as_ref();
        }

        todo!()
    }
}
