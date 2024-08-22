use std::{
    collections::{HashMap, VecDeque},
    ops::Deref,
    sync::Arc,
    time::Duration,
};

use futures::{lock::Mutex, AsyncReadExt, AsyncWriteExt, TryStreamExt};
use futures_map::KeyWaitMap;
use identity::{PeerId, PublicKey};
use multiaddr::Multiaddr;
use multistream_select::{dialer_select_proto, listener_select_proto, Version};
use protobuf::Message;
use rand::{seq::IteratorRandom, thread_rng};
use rasi::{task::spawn_ok, timer::TimeoutExt};

use crate::{
    book::{syscall::DriverPeerBook, ConnectionType, MemoryPeerBook, PeerBook, PeerInfo},
    keystore::{syscall::DriverKeyStore, KeyStore, MemoryKeyStore},
    multiaddr::ToSockAddr,
    proto::identity::Identity,
    transport::{syscall::DriverTransport, Connection, Listener, Stream, Transport},
    Error, Result,
};

/// protocol name of libp2p identity
pub const PROTOCOL_IPFS_ID: &str = "/ipfs/id/1.0.0";

/// protocol name of libp2p identity push
pub const PROTOCOL_IPFS_PUSH_ID: &str = "/ipfs/id/push/1.0.0";

/// protocol name of libp2p ping
pub const PROTOCOL_IPFS_PING: &str = "/ipfs/ping/1.0.0";

/// immutable context data for one switch.
struct ImmutableSwitch {
    /// The maximun size of active connections pool size.
    max_conn_pool_size: usize,
    /// The maximun length of inbound streams queue.
    max_inbound_length: usize,
    /// addresses that this switch is bound to.
    laddrs: Vec<Multiaddr>,
    /// The value of rpc timeout.
    timeout: Duration,
    /// A list of protocols that the switch accepts.
    protos: Vec<String>,
    /// This is a free-form string, identitying the implementation of the peer. The usual format is agent-name/version,
    /// where agent-name is the name of the program or library and version is its semantic version.
    agent_version: String,
    /// The max length of identity packet.
    max_identity_packet_size: usize,
    /// A list of transport that this switch registered.
    transports: Vec<Transport>,
    /// Keystore registered to this switch.
    keystore: KeyStore,
    /// Peer book for this switch.
    peer_book: PeerBook,
}

impl ImmutableSwitch {
    fn new(agent_version: String) -> Self {
        Self {
            max_conn_pool_size: 20,
            max_inbound_length: 200,
            agent_version,
            timeout: Duration::from_secs(10),
            protos: [PROTOCOL_IPFS_ID, PROTOCOL_IPFS_PUSH_ID, PROTOCOL_IPFS_PING]
                .into_iter()
                .map(|v| v.to_owned())
                .collect(),
            max_identity_packet_size: 4096,
            transports: vec![],
            keystore: MemoryKeyStore::random().into(),
            peer_book: MemoryPeerBook::default().into(),
            laddrs: vec![],
        }
    }

    fn get_transport_by_address(&self, laddr: &Multiaddr) -> Option<&Transport> {
        self.transports
            .iter()
            .find(|transport| transport.multiaddr_hit(laddr))
    }
}

/// Variant type used by [`connect`](Switch::connect) function.
pub enum ConnectTo<'a> {
    PeerIdRef(&'a PeerId),
    MultiaddrRef(&'a Multiaddr),
    PeerId(PeerId),
    Multiaddr(Multiaddr),
}

impl<'a> From<&'a PeerId> for ConnectTo<'a> {
    fn from(value: &'a PeerId) -> Self {
        Self::PeerIdRef(value)
    }
}

impl<'a> From<&'a Multiaddr> for ConnectTo<'a> {
    fn from(value: &'a Multiaddr) -> Self {
        Self::MultiaddrRef(value)
    }
}

impl From<PeerId> for ConnectTo<'static> {
    fn from(value: PeerId) -> Self {
        Self::PeerId(value)
    }
}

impl From<Multiaddr> for ConnectTo<'static> {
    fn from(value: Multiaddr) -> Self {
        Self::Multiaddr(value)
    }
}

/// A builder to create the `Switch` instance.
pub struct SwitchBuilder {
    ops: Result<ImmutableSwitch>,
}

impl SwitchBuilder {
    /// Set the `max_conn_pool_size`, the default value is `20`
    pub fn max_conn_pool_size(self, value: usize) -> Self {
        self.and_then(|mut cfg| {
            cfg.max_conn_pool_size = value;

            Ok(cfg)
        })
    }

    /// Set the `max_inbound_length`, the default value is `200`
    pub fn max_inbound_length(self, value: usize) -> Self {
        self.and_then(|mut cfg| {
            cfg.max_inbound_length = value;

            Ok(cfg)
        })
    }
    /// Replace default [`MemoryKeyStore`].
    pub fn keystore<K>(self, value: K) -> Self
    where
        K: DriverKeyStore + 'static,
    {
        self.and_then(|mut cfg| {
            cfg.keystore = value.into();

            Ok(cfg)
        })
    }

    /// Replace default [`MemoryPeerBook`].
    pub fn peer_book<R>(self, value: R) -> Self
    where
        R: DriverPeerBook + 'static,
    {
        self.and_then(|mut cfg| {
            cfg.peer_book = value.into();

            Ok(cfg)
        })
    }

    /// Set the protocol timeout, the default value is `10s`
    pub fn timeout(self, duration: Duration) -> Self {
        self.and_then(|mut cfg| {
            cfg.timeout = duration;

            Ok(cfg)
        })
    }

    /// Set the receive max buffer length of identity protocol.
    pub fn max_identity_packet_size(self, value: usize) -> Self {
        self.and_then(|mut cfg| {
            cfg.max_identity_packet_size = value;

            Ok(cfg)
        })
    }

    /// Register a new transport driver for the switch.
    pub fn transport<T>(self, value: T) -> Self
    where
        T: DriverTransport + 'static,
    {
        self.and_then(|mut cfg| {
            cfg.transports.push(value.into());

            Ok(cfg)
        })
    }

    /// Add a new listener which is bound to `laddr`.
    pub fn bind(self, laddr: Multiaddr) -> Self {
        self.and_then(|mut cfg| {
            cfg.laddrs.push(laddr);

            Ok(cfg)
        })
    }

    /// Set the protocol list of this switch accepts.
    pub fn protos<I>(self, value: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.and_then(|mut cfg| {
            let mut protos = value
                .into_iter()
                .map(|item| item.as_ref().to_owned())
                .collect::<Vec<_>>();

            cfg.protos.append(&mut protos);

            Ok(cfg)
        })
    }

    /// Consume the builder and create a new `Switch` instance.
    pub async fn create(self) -> Result<Switch> {
        let ops = self.ops?;

        let public_key = ops.keystore.public_key().await?;

        let switch = Switch {
            inner: Arc::new(InnerSwitch {
                local_peer_id: public_key.to_peer_id(),
                public_key,
                mutable: Mutex::new(MutableSwitch::new(ops.max_conn_pool_size)),
                immutable: ops,
                event_map: KeyWaitMap::new(),
            }),
        };

        for laddr in switch.immutable.laddrs.iter() {
            switch.listen(&laddr).await?;
        }

        Ok(switch)
    }

    fn and_then<F>(self, func: F) -> Self
    where
        F: FnOnce(ImmutableSwitch) -> Result<ImmutableSwitch>,
    {
        SwitchBuilder {
            ops: self.ops.and_then(func),
        }
    }
}

/// An in-memory connection pool.
#[derive(Default)]
struct ConnPool {
    max_pool_size: usize,
    /// mapping id => connection.
    conns: HashMap<String, Connection>,
    /// mapping peer_id to conn id.
    peers: HashMap<PeerId, Vec<String>>,
}

impl ConnPool {
    fn conn_pool_gc(&mut self) {
        log::trace!("conn_pool_gc, size={}", self.conns.len());

        if self.conns.len() < self.max_pool_size {
            return;
        }

        let mut removed = vec![];

        for (_, conn) in &self.conns {
            if conn.actives() == 0 {
                removed.push(conn.clone());
            }
        }

        log::trace!(
            "conn_pool_gc, size={}, removed={}",
            self.conns.len(),
            removed.len()
        );

        for conn in removed {
            self.remove(&conn);
        }
    }

    /// Put a new connecton instance into the pool, and update indexers.
    fn put(&mut self, conn: Connection) {
        self.conn_pool_gc();

        let peer_id = conn.public_key().to_peer_id();

        let raddr = conn
            .peer_addr()
            .to_sockaddr()
            .expect("Invalid transport peer_addr.");

        let id = conn.id().to_owned();

        // consistency test.
        if let Some(conn) = self.conns.get(&id) {
            let o_peer_id = conn.public_key().to_peer_id();

            let o_raddr = conn
                .peer_addr()
                .to_sockaddr()
                .expect("Invalid transport peer_addr.");

            assert_eq!(peer_id, o_peer_id, "consistency guarantee");
            assert_eq!(o_raddr, raddr, "consistency guarantee");

            return;
        }

        log::info!(target: "Switch","add new conn, id={}, raddr={}, peer={}",id , raddr, peer_id);

        self.conns.insert(id.to_owned(), conn);

        if let Some(conn_ids) = self.peers.get_mut(&peer_id) {
            conn_ids.push(id);
        } else {
            self.peers.insert(peer_id, vec![id]);
        }
    }

    fn get(&self, peer_id: &PeerId) -> Option<Vec<Connection>> {
        if let Some(conn_ids) = self.peers.get(&peer_id) {
            Some(
                conn_ids
                    .iter()
                    .map(|id| self.conns.get(id).expect("consistency guarantee").clone())
                    .collect(),
            )
        } else {
            None
        }
    }

    fn remove(&mut self, conn: &Connection) {
        let peer_id = conn.public_key().to_peer_id();

        let raddr = conn
            .peer_addr()
            .to_sockaddr()
            .expect("Invalid transport peer_addr.");

        let id = conn.id().to_owned();

        if let Some(conn) = self.conns.remove(&id) {
            let o_peer_id = conn.public_key().to_peer_id();

            let o_raddr = conn
                .peer_addr()
                .to_sockaddr()
                .expect("Invalid transport peer_addr.");

            assert_eq!(peer_id, o_peer_id, "consistency guarantee");
            assert_eq!(o_raddr, raddr, "consistency guarantee");
        }

        log::info!(target: "Switch","remove conn, id={}, raddr={}, peer={}",id , raddr, peer_id);

        if let Some(conn_ids) = self.peers.get_mut(&peer_id) {
            if let Some((index, _)) = conn_ids.iter().enumerate().find(|(_, v)| **v == id) {
                conn_ids.remove(index);
            }
        }
    }
}

#[derive(Default)]
struct MutableSwitch {
    conn_pool: ConnPool,
    incoming_streams: VecDeque<(Stream, String)>,
    laddrs: Vec<Multiaddr>,
}

impl MutableSwitch {
    fn new(max_pool_size: usize) -> Self {
        Self {
            conn_pool: ConnPool {
                max_pool_size,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum SwitchEvent {
    Accept,
}

#[doc(hidden)]
pub struct InnerSwitch {
    public_key: PublicKey,
    local_peer_id: PeerId,
    immutable: ImmutableSwitch,
    mutable: Mutex<MutableSwitch>,
    event_map: KeyWaitMap<SwitchEvent, ()>,
}

impl Drop for InnerSwitch {
    fn drop(&mut self) {
        log::trace!("Switch dropping.");
    }
}

/// `Switch` is the entry point of the libp2p network.
///
/// via `Switch` instance, you can:
/// - create a outbound stream to peer.
/// - accept a inbound stream from peer.
///
/// # Multiaddr hit
#[derive(Clone)]
pub struct Switch {
    inner: Arc<InnerSwitch>,
}

impl Deref for Switch {
    type Target = InnerSwitch;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Switch {
    async fn handle_incoming(&self, listener: Listener) -> Result<()> {
        let mut incoming = listener.into_incoming();

        while let Some(mut conn) = incoming.try_next().await? {
            log::trace!(target:"switch","accept a new incoming connection, peer={}, local={}", conn.peer_addr(),conn.local_addr());

            let this = self.clone();

            spawn_ok(async move {
                if let Err(err) = this.setup_conn(&mut conn).await {
                    log::error!(target:"switch","setup connection, peer={}, local={}, err={}", conn.peer_addr(),conn.local_addr(),err);
                    _ = conn.close(&this).await;
                } else {
                    log::trace!(target:"switch","setup connection, peer={}, local={}", conn.peer_addr(),conn.local_addr());

                    this.mutable.lock().await.conn_pool.put(conn);
                }
            })
        }

        todo!()
    }

    async fn setup_conn(&self, conn: &mut Connection) -> Result<()> {
        let this = self.clone();

        let mut this_conn = conn.clone();

        let peer_id = conn.public_key().to_peer_id();

        spawn_ok(async move {
            if let Err(err) = this.incoming_stream_loop(&mut this_conn).await {
                log::error!(target:"switch","incoming stream loop stopped, peer={}, local={}, error={}",this_conn.peer_addr(),this_conn.local_addr(),err);
                _ = this_conn.close(&this).await;
            } else {
                log::info!(target:"switch","incoming stream loop stopped, peer={}, local={}",this_conn.peer_addr(),this_conn.local_addr());
            }

            _ = this
                .update_conn_type(&peer_id, ConnectionType::CanConnect)
                .await;
        });

        // start "/ipfs/id/1.0.0" handshake.
        self.identity_request(conn)
            .timeout(self.immutable.timeout)
            .await
            .ok_or(Error::Timeout)??;

        Ok(())
    }

    async fn incoming_stream_loop(&self, conn: &mut Connection) -> Result<()> {
        loop {
            let stream = conn.accept().await?;

            let id = stream.id().to_owned();

            if let Err(err) = self.handle_incoming_stream(stream).await {
                log::error!(target:"switch","dispatch stream, id={}, err={}", id, err);
            }
        }
    }

    async fn handle_incoming_stream(&self, mut stream: Stream) -> Result<()> {
        log::info!(target:"switch","accept new stream, peer={}, local={}, id={}",stream.peer_addr(),stream.local_addr(),stream.id());

        let (protoco_id, _) = listener_select_proto(&mut stream, &self.immutable.protos)
            .timeout(self.immutable.timeout)
            .await
            .ok_or(Error::Timeout)??;

        log::info!(target:"switch","protocol handshake, id={}, protocol={}, peer_id={}",stream.id(),protoco_id, stream.public_key().to_peer_id());

        let this = self.clone();
        let protoco_id = protoco_id.clone();

        spawn_ok(async move {
            let peer_addr = stream.peer_addr().clone();
            let local_addr = stream.local_addr().clone();
            let id = stream.id().to_owned();

            if let Err(err) = this.dispatch_stream(protoco_id, stream).await {
                log::error!(target:"switch","dispatch stream, id={}, peer={}, local={}, err={}",id, peer_addr,local_addr,err);
            } else {
                log::trace!(target:"switch","dispatch stream ok, id={}, peer={}, local={}",id, peer_addr, local_addr);
            }
        });

        Ok(())
    }

    async fn dispatch_stream(&self, protoco_id: String, stream: Stream) -> Result<()> {
        let conn_peer_id = stream.public_key().to_peer_id();

        match protoco_id.as_str() {
            PROTOCOL_IPFS_ID => self.identity_response(stream).await?,
            PROTOCOL_IPFS_PUSH_ID => self.identity_push(&conn_peer_id, stream).await?,
            PROTOCOL_IPFS_PING => self.ping_echo(stream).await?,
            _ => {
                let mut mutable = self.mutable.lock().await;

                if mutable.incoming_streams.len() > self.immutable.max_inbound_length {
                    log::warn!(
                        "The maximun incoming queue size is reached({})",
                        self.immutable.max_inbound_length
                    );

                    return Ok(());
                }

                mutable.incoming_streams.push_back((stream, protoco_id));

                drop(mutable);

                self.event_map.insert(SwitchEvent::Accept, ());
            }
        }

        Ok(())
    }

    /// Handle `/ipfs/ping/1.0.0` request.
    async fn ping_echo(&self, mut stream: Stream) -> Result<()> {
        loop {
            log::trace!("recv /ipfs/ping/1.0.0");

            let body_len = unsigned_varint::aio::read_usize(&mut stream).await?;

            log::trace!("recv /ipfs/ping/1.0.0 payload len {}", body_len);

            if body_len != 32 {
                return Err(Error::InvalidPingLength(body_len));
            }

            let mut buf = vec![0; 31];

            stream.read_exact(&mut buf).await?;

            let mut payload_len = unsigned_varint::encode::usize_buffer();

            stream
                .write_all(unsigned_varint::encode::usize(buf.len(), &mut payload_len))
                .await?;

            stream.write_all(&buf).await?;

            log::trace!("send /ipfs/ping/1.0.0 echo");
        }
    }

    async fn identity_push(&self, conn_peer_id: &PeerId, mut stream: Stream) -> Result<()> {
        let identity = {
            log::trace!("identity_request: read varint length");

            let body_len = unsigned_varint::aio::read_usize(&mut stream).await?;

            log::trace!("identity_request: read varint length");

            if self.immutable.max_identity_packet_size < body_len {
                return Err(Error::IdentityOverflow(
                    self.immutable.max_identity_packet_size,
                ));
            }

            log::trace!("identity_request recv body: {}", body_len);

            let mut buf = vec![0; body_len];

            stream.read_exact(&mut buf).await?;

            Identity::parse_from_bytes(&buf)?
        };

        let pubkey = PublicKey::try_decode_protobuf(identity.publicKey())?;

        let peer_id = pubkey.to_peer_id();

        if *conn_peer_id != peer_id {
            return Err(Error::IdentityCheckFailed(*conn_peer_id, peer_id));
        }

        let raddrs = identity
            .listenAddrs
            .into_iter()
            .map(|buf| Multiaddr::try_from(buf).map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;

        let observed_addrs = identity
            .observedAddr
            .into_iter()
            .map(|buf| Multiaddr::try_from(buf).map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;

        //TODO: add nat codes
        log::info!(target:"switch","{} observed addrs: {:?}", peer_id, observed_addrs);

        let peer_info = PeerInfo {
            id: peer_id,
            addrs: raddrs,
            conn_type: ConnectionType::Connected,
        };

        self.update_peer_info(peer_info).await?;

        Ok(())
    }

    /// Start a "/ipfs/id/1.0.0" handshake.
    async fn identity_request(&self, conn: &mut Connection) -> Result<()> {
        let mut stream = conn.connect().await?;

        let conn_peer_id = conn.public_key().to_peer_id();

        dialer_select_proto(&mut stream, ["/ipfs/id/1.0.0"], Version::V1).await?;

        self.identity_push(&conn_peer_id, stream).await
    }

    async fn identity_response(&self, mut stream: Stream) -> Result<()> {
        log::trace!("handle identity request");

        let peer_addr = stream.peer_addr();

        let mut identity = Identity::new();

        identity.set_observedAddr(peer_addr.to_vec());

        identity.set_publicKey(self.public_key().encode_protobuf());

        identity.set_agentVersion(self.immutable.agent_version.to_owned());

        identity.listenAddrs = self
            .local_addrs()
            .await
            .iter()
            .map(|addr| addr.to_vec())
            .collect::<Vec<_>>();

        identity.protocols = self.immutable.protos.clone();

        let buf = identity.write_to_bytes()?;

        log::trace!(
            "handle identity request, protos={:?}",
            self.immutable.protos
        );

        let mut payload_len = unsigned_varint::encode::usize_buffer();

        stream
            .write_all(unsigned_varint::encode::usize(buf.len(), &mut payload_len))
            .await?;

        stream.write_all(&buf).await?;

        Ok(())
    }

    /// Create a new transport listener with provided `laddr`.
    async fn listen(&self, laddr: &Multiaddr) -> Result<()> {
        let transport = self
            .immutable
            .get_transport_by_address(laddr)
            .ok_or(Error::UnspportMultiAddr(laddr.to_owned()))?;

        let listener = transport.bind(laddr, self.clone()).await?;

        let laddr = listener.local_addr()?;

        self.mutable.lock().await.laddrs.push(laddr.clone());

        let this = self.clone();

        spawn_ok(async move {
            if let Err(err) = this.handle_incoming(listener).await {
                log::error!(target:"switch" ,"listener({}) stop, err={}",laddr, err);
            } else {
                log::info!(target:"switch" ,"listener({}) stop",laddr);
            }
        });

        Ok(())
    }

    /// Connect to peer with provided [`raddr`](Multiaddr).
    ///
    /// This function first query the route table to get the peer id,
    /// if exists then check for a local connection cache.
    ///
    async fn connect_peer_to(&self, raddr: &Multiaddr) -> Result<Connection> {
        if let Some(peer_id) = self.peer_id_of(raddr).await? {
            return self.connect_peer(&peer_id).await;
        }

        self.connect_peer_to_prv(raddr).await
    }

    async fn connect_peer_to_prv(&self, raddr: &Multiaddr) -> Result<Connection> {
        let transport = self
            .immutable
            .get_transport_by_address(raddr)
            .ok_or(Error::UnspportMultiAddr(raddr.to_owned()))?;

        let mut conn = transport.connect(raddr, self.clone()).await?;

        if let Err(err) = self.setup_conn(&mut conn).await {
            log::error!(target:"switch","setup connection, peer={}, local={}, err={}",conn.peer_addr(),conn.local_addr(),err);
        } else {
            self.mutable.lock().await.conn_pool.put(conn.clone());
        }

        Ok(conn)
    }

    /// Create a new connection to peer by id.
    ///
    /// This function will first check for a local connection cache,
    /// and if there is one, it will directly return the cached connection
    async fn connect_peer(&self, id: &PeerId) -> Result<Connection> {
        if let Some(conns) = self.mutable.lock().await.conn_pool.get(id) {
            if !conns.is_empty() {
                let conn = conns.into_iter().choose(&mut thread_rng()).unwrap();
                log::trace!("connection reused, id={}, conn_id={}", id, conn.id());
                return Ok(conn);
            }
        }

        let raddrs = self
            .immutable
            .peer_book
            .get(id)
            .await?
            .ok_or(Error::ConnectPeer(id.clone()))?;

        let mut last_error = None;

        for raddr in raddrs.addrs {
            log::trace!("connect, id={}, raddr={}", id, raddr);
            match self.connect_peer_to_prv(&raddr).await {
                Ok(conn) => {
                    log::trace!("connected, id={}, raddr={}", id, raddr);
                    return Ok(conn);
                }
                Err(err) => {
                    last_error = {
                        log::trace!("connect failed, id={}, raddr={}, err={}", id, raddr, err);
                        Some(err)
                    }
                }
            }
        }

        self.update_conn_type(id, ConnectionType::CannotConnect)
            .await?;

        Err(last_error.unwrap_or(Error::ConnectPeer(id.to_owned())))
    }

    pub(crate) async fn remove_conn(&self, conn: &Connection) {
        _ = self.mutable.lock().await.conn_pool.remove(conn);
    }
}

impl Switch {
    /// Uses `agent_version` string to create a switch [`builder`](SwitchBuilder).
    pub fn new<A>(agent_version: A) -> SwitchBuilder
    where
        A: AsRef<str>,
    {
        SwitchBuilder {
            ops: Ok(ImmutableSwitch::new(agent_version.as_ref().to_owned())),
        }
    }

    /// Create a new stream to `target` with provided `protos`.
    pub async fn connect<'a, C, I>(&self, target: C, protos: I) -> Result<(Stream, String)>
    where
        C: Into<ConnectTo<'a>>,
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut conn = match target.into() {
            ConnectTo::PeerIdRef(peer_id) => self.connect_peer(peer_id).await?,
            ConnectTo::MultiaddrRef(raddr) => self.connect_peer_to(raddr).await?,
            ConnectTo::PeerId(peer_id) => self.connect_peer(&peer_id).await?,
            ConnectTo::Multiaddr(raddr) => self.connect_peer_to(&raddr).await?,
        };

        let mut stream = conn.connect().await?;

        let (protocol_id, _) = dialer_select_proto(&mut stream, protos, Version::V1)
            .timeout(self.immutable.timeout)
            .await
            .ok_or(Error::Timeout)??;

        Ok((stream, protocol_id.as_ref().to_owned()))
    }

    /// Accept a new incoming stream.
    ///
    ///
    /// # Take over the handle of incoming stream
    ///
    /// If this instance belongs to [`ServeMux`](crate::serve::ServeMux),
    /// this function or [`into_incoming`](Self::into_incoming) should not be called.
    pub async fn accept(&self) -> Result<(Stream, String)> {
        loop {
            let mut mutable = self.mutable.lock().await;

            if let Some(r) = mutable.incoming_streams.pop_front() {
                return Ok(r);
            }

            self.event_map.wait(&SwitchEvent::Accept, mutable).await;
        }
    }

    /// Conver the switch into a [`Stream`](futures::Stream) object.
    pub fn into_incoming(self) -> impl futures::Stream<Item = Result<(Stream, String)>> + Unpin {
        Box::pin(futures::stream::unfold(self, |listener| async move {
            let res = listener.accept().await;
            Some((res, listener))
        }))
    }

    /// Update the [`PeerBook`] of this switch.
    pub async fn update_peer_info(&self, peer_info: PeerInfo) -> Result<Option<PeerInfo>> {
        Ok(self.immutable.peer_book.put(peer_info).await?)
    }
}

impl Switch {
    /// Remove [`PeerInfo`] from the [`PeerBook`] of this switch.
    pub async fn remove_peer_info(&self, peer_id: &PeerId) -> Result<Option<PeerInfo>> {
        Ok(self.immutable.peer_book.remove(peer_id).await?)
    }

    /// Update the connection type of the [`peer_id`](PeerId) in the [`PeerBook`] of this switch.
    pub async fn update_conn_type(
        &self,
        peer_id: &PeerId,
        conn_type: ConnectionType,
    ) -> Result<Option<ConnectionType>> {
        Ok(self
            .immutable
            .peer_book
            .update_conn_type(peer_id, conn_type)
            .await?)
    }

    /// Returns the [`PeerInfo`] of the [`peer_id`](PeerId).
    pub async fn peer_info(&self, peer_id: &PeerId) -> Result<Option<PeerInfo>> {
        Ok(self.immutable.peer_book.get(peer_id).await?)
    }

    /// Returns the [`PeerId`] of the [`raddr`](Multiaddr).
    pub async fn peer_id_of(&self, raddr: &Multiaddr) -> Result<Option<PeerId>> {
        Ok(self.immutable.peer_book.peer_id_of(raddr).await?)
    }

    /// Returns the public key of this switch.
    ///
    /// This returned value is provided by [`KeyStore`] service.
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Get associated keystore instance.
    pub fn keystore(&self) -> &KeyStore {
        &self.immutable.keystore
    }

    /// Get this switch's public key.
    pub fn local_public_key(&self) -> &PublicKey {
        &self.inner.public_key
    }

    /// Get this switch's node id.
    pub fn local_id(&self) -> &PeerId {
        &self.inner.local_peer_id
    }

    /// Returns the addresses list of this switch is bound to.
    pub async fn local_addrs(&self) -> Vec<Multiaddr> {
        self.mutable.lock().await.laddrs.clone()
    }
}
