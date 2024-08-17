use async_trait::async_trait;
use futures::lock::Mutex;
use identity::PeerId;
use rep2p::driver_wrapper;

use crate::primitives::{KBucketTable, Key, PeerInfo};

/// A kad peer store driver must implement the `Driver-*` traits in this module.
pub mod syscall {
    use async_trait::async_trait;
    use identity::PeerId;

    use crate::primitives::PeerInfo;

    /// A trait that provides functions to access peer informations.
    #[async_trait]
    pub trait DriverKadRouteTable {
        /// Returns the replication parameter(`k`).
        fn const_k(&self) -> usize;
        /// insert new peer informations.
        async fn insert(&self, info: PeerInfo) -> std::io::Result<()>;

        /// Remove peer information from route table.
        async fn remove(&self, id: &PeerId) -> std::io::Result<()>;

        /// Get the about up to [`K`](DriverKadRouteTable::k) closest nodes's [`PeerInfo`]
        async fn closest(&self, id: &PeerId) -> std::io::Result<Vec<PeerInfo>>;
    }
}

driver_wrapper!(
    ["A type wrapper of [`DriverKadStore`](syscall::DriverKadStore)"]
    KadRouteTable[syscall::DriverKadRouteTable]
);

pub struct KBucketRouteTable(Mutex<KBucketTable>);

impl KBucketRouteTable {
    pub fn new(local_id: &PeerId) -> Self {
        KBucketRouteTable(Mutex::new(KBucketTable::new(Key::from(local_id))))
    }
}

#[async_trait]
impl syscall::DriverKadRouteTable for KBucketRouteTable {
    /// Returns the replication parameter(`k`).
    fn const_k(&self) -> usize {
        20
    }
    /// insert new peer informations.
    async fn insert(&self, info: PeerInfo) -> std::io::Result<()> {
        self.0.lock().await.insert(info.id, info);

        Ok(())
    }

    /// Remove peer information from route table.
    async fn remove(&self, id: &PeerId) -> std::io::Result<()> {
        self.0.lock().await.remove(&Key::from(id));

        Ok(())
    }

    /// Get the about up to [`K`](DriverKadRouteTable::k) closest nodes's [`PeerInfo`]
    async fn closest(&self, id: &PeerId) -> std::io::Result<Vec<PeerInfo>> {
        Ok(self
            .0
            .lock()
            .await
            .closest_k(&Key::from(id))
            .map(|(_, info)| info.clone())
            .collect())
    }
}
