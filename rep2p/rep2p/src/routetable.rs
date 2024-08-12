//! A utility to handle peer routing information, used by [`Switch`](super::Switch).

use std::{collections::HashMap, io::Result, sync::Mutex};

use async_trait::async_trait;
use identity::PeerId;
use multiaddr::Multiaddr;

use crate::driver_wrapper;

/// A libp2p keystore driver must implement the `Driver-*` traits in this module.
pub mod syscall {
    use std::io::Result;

    use async_trait::async_trait;
    use futures::Stream;
    use identity::PeerId;
    use multiaddr::Multiaddr;

    use super::RouteTableStream;

    #[async_trait]
    pub trait DriverRouteTable: Sync + Send {
        /// Add addresses to route table by `peer_id`.
        async fn put(&self, peer_id: PeerId, addrs: &[Multiaddr]) -> Result<()>;
        /// Get peer's listener address.
        ///
        /// On success, returns a asynchronous [`Stream`] of listener's addresses.
        async fn get(&self, peer_id: &PeerId) -> Result<RouteTableStream>;

        /// Delete `addrs` from route table by `peer_id`.
        async fn delete(&self, peer_id: &PeerId) -> Result<()>;

        /// Get the peer id by address.
        async fn peer_id_of(&self, addr: &Multiaddr) -> Result<Option<PeerId>>;
    }

    pub trait DriverRouteTableStream:
        Stream<Item = Result<Multiaddr>> + Sync + Send + Unpin
    {
    }

    impl<T> DriverRouteTableStream for T where T: Stream<Item = Result<Multiaddr>> + Sync + Send + Unpin {}
}

driver_wrapper!(
    ["A type wrapper of [`DriverRouteTable`](syscall::DriverRouteTable)"]
    RouteTable[syscall::DriverRouteTable]
);

driver_wrapper!(
    ["A type wrapper of [`DriverRouteTableStream`](syscall::DriverRouteTableStream)"]
    RouteTableStream[syscall::DriverRouteTableStream]
);

#[derive(Default)]
struct RawMemoryRouteTable {
    peers: HashMap<PeerId, Vec<Multiaddr>>,
    addrs: HashMap<Multiaddr, PeerId>,
}

/// An in memory [`RouteTable`] implementation
#[derive(Default)]
pub struct MemoryRouteTable(Mutex<RawMemoryRouteTable>);

#[async_trait]
impl syscall::DriverRouteTable for MemoryRouteTable {
    /// Add addresses to route table by `peer_id`.
    async fn put(&self, peer_id: PeerId, addrs: &[Multiaddr]) -> Result<()> {
        log::trace!("peer={}, update route, addrs={:?}", peer_id, addrs);

        let addrs = addrs.to_vec();
        let mut raw = self.0.lock().unwrap();

        if let Some(old_addrs) = raw.peers.remove(&peer_id) {
            for addr in old_addrs {
                raw.addrs.remove(&addr);
            }
        }

        for addr in &addrs {
            raw.addrs.insert(addr.to_owned(), peer_id.to_owned());
        }

        raw.peers.insert(peer_id, addrs);

        Ok(())
    }
    /// Get peer's listener address.
    ///
    /// On success, returns a asynchronous `Stream` of listener's addresses.
    async fn get(&self, peer_id: &PeerId) -> Result<RouteTableStream> {
        let raw = self.0.lock().unwrap();

        if let Some(addrs) = raw.peers.get(peer_id) {
            let addrs: Vec<Result<Multiaddr>> =
                addrs.iter().cloned().map(|addr| Ok(addr)).collect();

            return Ok(RouteTableStream::from(futures::stream::iter(addrs)));
        }

        todo!()
    }

    /// Delete `addrs` from route table by `peer_id`.
    async fn delete(&self, peer_id: &PeerId) -> Result<()> {
        let mut raw = self.0.lock().unwrap();

        if let Some(old_addrs) = raw.peers.remove(&peer_id) {
            for addr in old_addrs {
                raw.addrs.remove(&addr);
            }
        }

        Ok(())
    }

    /// Get the peer id by address.
    async fn peer_id_of(&self, addr: &Multiaddr) -> Result<Option<PeerId>> {
        let raw = self.0.lock().unwrap();

        Ok(raw.addrs.get(addr).map(|id| id.clone()))
    }
}
