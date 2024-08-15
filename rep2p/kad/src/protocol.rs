//! This module provide a rep2p compatibable kad implementation.

use std::sync::Arc;

use futures::lock::Mutex;
use identity::PeerId;
use rep2p::{multiaddr::Multiaddr, Switch};

use crate::primitives::{KBucketTable, Key};

/// protocol name of libp2p kad.
pub const PROTOCOL_IPFS_KAD: &str = "/ipfs/kad/1.0.0";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Multiaddr without p2p node, {0}")]
    InvalidSeedMultAddr(Multiaddr),
}

/// Result type returns by this module functionss.
pub type Result<T> = std::result::Result<T, Error>;

#[allow(unused)]
/// A variant for kademlia network operations.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum KadOps {
    FindNode(Key),
    GetValue(Key),
    PutValue(Key),
    AddProvider(Key),
    GetProviders(Key),
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
    pub async fn new<S>(switch: &Switch, seeds: S) -> Result<Self>
    where
        S: IntoIterator,
        S::Item: AsRef<Multiaddr>,
    {
        let mut k_bucket_table = KBucketTable::new(Key::from(switch.local_id()));

        log::info!("start kademlia node bootstrap process..");

        for raddr in seeds.into_iter() {
            let raddr = raddr.as_ref();

            match raddr
                .clone()
                .pop()
                .ok_or_else(|| Error::InvalidSeedMultAddr(raddr.clone()))?
            {
                rep2p::multiaddr::Protocol::P2p(id) => {
                    k_bucket_table.insert(id, raddr.clone());
                }
                _ => {
                    return Err(Error::InvalidSeedMultAddr(raddr.clone()));
                }
            }
        }

        Ok(Self {
            switch: switch.clone(),
            k_bucket_table: Arc::new(Mutex::new(k_bucket_table)),
        })
    }
}
