use std::{
    collections::HashSet,
    fmt::Display,
    future::Future,
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::{
    channel::mpsc::{channel, Receiver},
    lock::Mutex,
    SinkExt, StreamExt,
};
use identity::PeerId;
use rasi::{task::spawn_ok, timer::TimeoutExt};
use rep2p::{book::PeerInfo, transport::Stream, Switch};

use crate::{
    errors::Result, kbucket::KBucketKey, primitives::Key, rpc::KadRpc, KadSwitch,
    PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD,
};

#[derive(Debug)]
/// A variant returns by query function.
pub enum Recursive {
    Removed(PeerId),
    /// the set of next query candidates,
    Next(PeerId, Vec<PeerInfo>),
    /// stop the recursive routing.
    Break(PeerId, Option<Vec<PeerInfo>>),
}

/// The context data for [`query`](Query::query) fn.
#[derive(Debug)]
pub struct RoutingContext {
    /// The counter of result k closest nodes.
    pub closest: usize,
    /// The counter of peers we've already queried.
    pub queried: usize,
    /// The counter of remaining query candidates.
    pub candidates: usize,
    /// The point in time at which the routing process began.
    pub start_instant: Instant,
}

impl Display for RoutingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "closest={}, queried={}, candidates={}, elapsed={:?}",
            self.closest,
            self.queried,
            self.candidates,
            self.start_instant.elapsed()
        )
    }
}

/// A recursive query type.
pub trait Query {
    fn key(&self) -> &Key;

    fn query(
        &self,
        cx: &RoutingContext,
        peer_info: PeerInfo,
    ) -> impl Future<Output = Recursive> + Send + 'static;
}

/// A context to execute recursive routings process.
pub struct Router<'a, Q> {
    /// The created timestamp of this router.
    start_instant: Instant,
    /// A kad protcol stack that this routing process executes on.
    switch: &'a KadSwitch,
    /// The result k closest nodes.
    closest_k: Vec<PeerId>,
    /// The track set of peers we've already queried.
    queried: HashSet<PeerId>,
    /// The set of next query candidates
    candidates: Vec<PeerId>,
    /// query context for this router.
    query: Q,
}

impl<'a, Q> Router<'a, Q>
where
    Q: Query,
{
    pub fn new(value: &'a KadSwitch, query: Q) -> Self {
        Self {
            start_instant: Instant::now(),
            switch: value,
            closest_k: Default::default(),
            queried: Default::default(),
            candidates: Default::default(),
            query,
        }
    }

    fn to_context(&self) -> RoutingContext {
        RoutingContext {
            closest: self.closest_k.len(),
            candidates: self.candidates.len(),
            queried: self.queried.len(),
            start_instant: self.start_instant,
        }
    }

    /// Start route with `alpha` concurrency parameter and custome `route` fn.
    pub async fn route(mut self, alpha: NonZeroUsize) -> Result<(Q, Vec<PeerId>)> {
        let alpha: usize = alpha.into();

        let mut candidates = vec![];

        for peer_id in self.switch.route_table.closest(self.query.key()).await? {
            if let Some(peer_info) = self.switch.switch.peer_info(&peer_id).await? {
                candidates.push(peer_info);
            }
        }

        self.add_candidates(candidates).await?;

        let (sender, mut receiver) = channel::<Recursive>(alpha);

        let mut pending = 0usize;

        while let Some(peer_id) = self.candidates.pop() {
            // check queried set.
            if self.queried.insert(peer_id.clone()) {
                // query the peer_info
                if let Some(peer_info) = self.switch.switch.peer_info(&peer_id).await? {
                    log::trace!("quering, id={}", peer_info.id);

                    // check if this peer_id is closer than ids in closest set.
                    if self.is_closer(&peer_id) {
                        let cx = self.to_context();

                        let fut = self.query.query(&cx, peer_info);

                        let mut sender = sender.clone();

                        spawn_ok(async move {
                            sender.send(fut.await).await.expect("Close receiver early");
                        });

                        pending += 1;
                    } else {
                        log::trace!(
                            "skip quering, id={}, reason='is not closer than found peers.'",
                            peer_id
                        )
                    }
                } else {
                    log::trace!(
                        "skip quering, id={}, reason='peer infomation not found.'",
                        peer_id
                    )
                }
            } else {
                log::trace!("skip quering, id={}, reason='already queried.'", peer_id)
            }

            log::trace!(
                "routing, alpha={}, pending={}, candidates={}",
                alpha,
                pending,
                self.candidates.len()
            );

            // Maximum concurrent lookup limit is reached.
            while (pending > 0 && self.candidates.len() == 0) || pending == alpha {
                let stop = self.wait_one(&mut receiver).await?;
                if stop {
                    return Ok((self.query, self.closest_k));
                }

                pending -= 1;
            }
        }

        assert_eq!(
            pending, 0,
            "check: while (pending > 0 && self.candidates.len() == 0) || pending == alpha"
        );

        return Ok((self.query, self.closest_k));
    }

    async fn wait_one(&mut self, receiver: &mut Receiver<Recursive>) -> Result<bool> {
        log::trace!("routing, wait one task...");

        let routing = receiver
            .next()
            .await
            .expect("There is always a sender owned by the current task.");

        log::trace!("routing, one task completed.");

        match routing {
            Recursive::Next(queried, peers) => {
                self.add_closest_k(queried);
                self.add_candidates(peers).await?;
            }
            Recursive::Removed(peer_id) => {
                log::error!("remove candidate peer, id={}", peer_id);
            }
            Recursive::Break(queried, Some(peers)) => {
                self.add_closest_k(queried);
                self.add_candidates(peers).await?;
                return Ok(true);
            }
            Recursive::Break(queried, None) => {
                self.add_closest_k(queried);
                return Ok(true);
            }
        }

        return Ok(false);
    }

    fn add_closest_k(&mut self, peer_id: PeerId) {
        self.closest_k.push(peer_id);

        self.closest_k.sort_by(|lhs, rhs| {
            let lhs = Key::from(lhs).distance(self.query.key());
            let rhs = Key::from(rhs).distance(self.query.key());

            lhs.cmp(&rhs)
        });

        let const_k = self.switch.route_table.const_k();

        if self.closest_k.len() > const_k {
            self.closest_k.truncate(const_k);
        }

        log::trace!("closest_k: {}", self.closest_k.len());
    }

    fn is_closer(&self, peer_id: &PeerId) -> bool {
        if self.closest_k.len() < self.switch.route_table.const_k() {
            return true;
        }

        if let Some(last) = self.closest_k.last() {
            let last_distance = Key::from(last).distance(self.query.key());
            let distance = Key::from(peer_id).distance(self.query.key());

            distance < last_distance
        } else {
            true
        }
    }

    async fn add_candidates(&mut self, peers: Vec<PeerInfo>) -> Result<()> {
        let max_distance = if let Some(last) = self.closest_k.last() {
            Some(Key::from(last).distance(self.query.key()))
        } else {
            None
        };

        for peer in peers {
            if self.queried.contains(&peer.id) {
                continue;
            }

            self.switch.switch.update_peer_info(peer.clone()).await?;

            let distance = Key::from(peer.id).distance(self.query.key());

            if let Some(max_distance) = &max_distance {
                if distance < *max_distance {
                    self.candidates.push(peer.id.clone());
                }
            } else {
                self.candidates.push(peer.id.clone());
            }
        }

        log::trace!("candidates: len={:?}", self.candidates.len());

        Ok(())
    }
}

async fn connect(switch: Switch, peer_info: &PeerInfo, timeout: Duration) -> Option<Stream> {
    for raddr in &peer_info.addrs {
        log::trace!(
            "quering, id={}, raddr={}, timeout={:?}",
            peer_info.id,
            raddr,
            timeout
        );

        match switch
            .connect(raddr, [PROTOCOL_IPFS_KAD, PROTOCOL_IPFS_LAN_KAD])
            .timeout(timeout)
            .await
        {
            Some(Ok((stream, _))) => return Some(stream),
            Some(Err(err)) => {
                log::error!(
                    "connect to peer, id={}, raddr={}, err={}",
                    peer_info.id,
                    raddr,
                    err
                );
            }
            _ => {
                log::error!(
                    "connect to peer, id={}, raddr={}, err='timeout({:?})'",
                    peer_info.id,
                    raddr,
                    timeout
                );
            }
        }
    }

    None
}

/// FIND_NODE recursive query.
pub(crate) struct FindNode<'a> {
    switch: &'a Switch,
    peer_id: &'a PeerId,
    key: Key,
    target: Arc<Mutex<Option<PeerInfo>>>,
    timeout: Duration,
    max_packet_len: usize,
}

impl<'a> FindNode<'a> {
    pub(crate) fn new(
        switch: &'a Switch,
        max_packet_len: usize,
        peer_id: &'a PeerId,
        timeout: Duration,
    ) -> Self {
        Self {
            max_packet_len,
            key: Key::from(peer_id),
            switch,
            peer_id,
            target: Default::default(),
            timeout,
        }
    }

    pub(crate) async fn into_peer_info(self) -> Option<PeerInfo> {
        self.target.lock().await.take()
    }
}

impl<'a> Query for FindNode<'a> {
    fn key(&self) -> &Key {
        &self.key
    }

    fn query(
        &self,
        cx: &RoutingContext,
        peer_info: PeerInfo,
    ) -> impl Future<Output = Recursive> + Send + 'static {
        log::trace!("query={}, target={}, {}", peer_info.id, self.peer_id, cx);

        let switch = self.switch.clone();
        let peer_id = self.peer_id.clone();
        let target = self.target.clone();
        let timeout = self.timeout;
        let max_packet_len = self.max_packet_len;

        async move {
            if let Some(stream) = connect(switch, &peer_info, timeout).await {
                match stream
                    .find_node(&peer_id, max_packet_len)
                    .timeout(timeout)
                    .await
                {
                    Some(Ok(candidates)) => {
                        log::trace!(
                            "query={}, target={}, resp={}",
                            peer_info.id,
                            peer_id,
                            candidates.len()
                        );

                        if let Some(info) = candidates.iter().find(|info| info.id == peer_id) {
                            *target.lock().await = Some(info.clone());

                            return Recursive::Break(peer_info.id, None);
                        }

                        return Recursive::Next(peer_info.id, candidates);
                    }
                    Some(Err(err)) => {
                        log::error!("query={}, target={}, err={}", peer_info.id, peer_id, err);
                        return Recursive::Removed(peer_info.id);
                    }
                    _ => {
                        log::error!(
                            "query={}, target={}, err='timeout({:?})'",
                            peer_info.id,
                            peer_id,
                            timeout
                        );
                        return Recursive::Removed(peer_info.id);
                    }
                }
            } else {
                return Recursive::Removed(peer_info.id);
            }
        }
    }
}
