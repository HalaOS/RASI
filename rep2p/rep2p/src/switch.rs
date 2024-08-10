use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{AsyncReadExt, AsyncWriteExt, TryStreamExt};
use identity::{PeerId, PublicKey};
use multiaddr::Multiaddr;
use multistream_select::{dialer_select_proto, listener_select_proto, Version};
use protobuf::Message;
use rasi::{task::spawn_ok, timer::TimeoutExt};

use crate::{
    keystore::{syscall::DriverKeyStore, KeyStore},
    proto::identity::Identity,
    protocol::{syscall::DriverProtocol, Protocol, ProtocolHandler},
    routetable::{syscall::DriverRouteTable, RouteTable},
    transport::{
        syscall::{DriverConnection, DriverTransport},
        Connection, Listener, Stream, Transport,
    },
    Error, Result,
};

const PROTOCOL_IPFS_ID: &str = "/ipfs/id/1.0.0";
const PROTOCOL_IPFS_PUSH_ID: &str = "/ipfs/id/push/1.0.0";
const PROTOCOL_IPFS_PING: &str = "/ipfs/ping/1.0.0";

/// immutable context data for one switch.
struct ImmutableSwitch {
    laddrs: Vec<Multiaddr>,
    /// The value of rpc timeout.
    timeout: Duration,
    /// A list of protocols that the switch accepts.
    protos: Vec<String>,
    /// Created protocol handlers.
    proto_handlers: HashMap<String, ProtocolHandler>,
    /// This is a free-form string, identitying the implementation of the peer. The usual format is agent-name/version,
    /// where agent-name is the name of the program or library and version is its semantic version.
    agent_version: String,
    /// The max length of identity packet.
    max_identity_len: usize,
    /// A list of transport that this switch registered.
    transports: Vec<Transport>,
    /// Keystore registered to this switch.
    keystore: KeyStore,
    /// RouteTable registered to this switch.
    route_table: RouteTable,
}

impl ImmutableSwitch {
    fn new(agent_version: String, keystore: KeyStore, route_table: RouteTable) -> Self {
        Self {
            agent_version,
            timeout: Duration::from_secs(10),
            protos: [PROTOCOL_IPFS_ID, PROTOCOL_IPFS_PUSH_ID, PROTOCOL_IPFS_PING]
                .into_iter()
                .map(|v| v.to_owned())
                .collect(),
            proto_handlers: Default::default(),
            max_identity_len: 4096,
            transports: vec![],
            keystore,
            route_table,
            laddrs: vec![],
        }
    }

    fn get_transport_by_address(&self, laddr: &Multiaddr) -> Option<&Transport> {
        self.transports
            .iter()
            .find(|transport| transport.is_support(laddr))
    }
}

struct MutableSwitch {
    conn_pool: HashMap<PeerId, Box<dyn DriverConnection>>,
}

impl MutableSwitch {
    fn new() -> Self {
        Self {
            conn_pool: HashMap::new(),
        }
    }

    fn put_conn(&mut self, conn: Box<dyn DriverConnection>) -> Result<()> {
        self.conn_pool.insert(conn.public_key()?.to_peer_id(), conn);

        Ok(())
    }

    fn get_conn(&mut self, id: &PeerId) -> Option<Box<dyn DriverConnection>> {
        self.conn_pool.remove(id)
    }
}

/// A builder to create the `Switch` instance.
pub struct SwitchBuilder {
    ops: Result<ImmutableSwitch>,
}

impl SwitchBuilder {
    /// Set the protocol timeout, the default value is `10s`
    pub fn timeout(self, duration: Duration) -> Self {
        self.and_then(|mut cfg| {
            cfg.timeout = duration;

            Ok(cfg)
        })
    }

    /// Set the receive max buffer length of identity protocol.
    pub fn max_identity_len(self, value: usize) -> Self {
        self.and_then(|mut cfg| {
            cfg.max_identity_len = value;

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
    pub fn bind<T>(self, laddr: Multiaddr) -> Self {
        self.and_then(|mut cfg| {
            cfg.laddrs.push(laddr);

            Ok(cfg)
        })
    }

    /// Set the protocol list of this switch accepts.
    pub fn proto<P>(self, value: P) -> Self
    where
        P: DriverProtocol + 'static,
    {
        self.and_then(|mut cfg| {
            let protocol: Protocol = value.into();

            cfg.proto_handlers
                .insert(protocol.name().to_owned(), protocol.create()?);

            cfg.protos.push(protocol.name().to_owned());

            Ok(cfg)
        })
    }

    /// Consume the builder and create a new `Switch` instance.
    pub async fn create(self) -> Result<Switch> {
        let ops = self.ops?;

        let public_key = ops.keystore.public_key().await?;

        let switch = Switch {
            public_key: Arc::new(public_key),
            immutable: Arc::new(ops),
            mutable: Arc::new(Mutex::new(MutableSwitch::new())),
        };

        for laddr in switch.immutable.laddrs.iter() {
            switch.listen(&laddr).await?;
        }

        for handler in switch.immutable.proto_handlers.values() {
            handler.start(&switch).await?;
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

/// `Switch` is the protocol stack entry point of the libp2p network.
#[derive(Clone)]
pub struct Switch {
    public_key: Arc<PublicKey>,
    immutable: Arc<ImmutableSwitch>,
    mutable: Arc<Mutex<MutableSwitch>>,
}

impl Switch {
    async fn handle_incoming(&self, listener: Listener) -> Result<()> {
        let mut incoming = listener.into_incoming();

        while let Some(mut conn) = incoming.try_next().await? {
            let peer_addr = conn.peer_addr()?;
            let local_addr = conn.local_addr()?;
            log::trace!(target:"switch","accept a new incoming connection, peer={}, local={}", peer_addr,local_addr);

            let this = self.clone();

            spawn_ok(async move {
                if let Err(err) = this.setup_conn(&mut conn).await {
                    log::error!(target:"switch","connection stop, peer={}, local={}, err={}", peer_addr,local_addr,err);
                    _ = conn.close().await;
                } else {
                    log::trace!(target:"switch","connection stop, peer={}, local={}", peer_addr,local_addr);
                }
            })
        }

        todo!()
    }

    async fn setup_conn(&self, conn: &mut Connection) -> Result<()> {
        let this = self.clone();

        let peer_addr = conn.peer_addr()?;
        let local_addr = conn.local_addr()?;

        let mut this_conn = conn.try_clone()?;

        spawn_ok(async move {
            if let Err(err) = this.handle_incoming_stream(&mut this_conn).await {
                log::error!(target:"switch","handle incoming stream, peer={}, local={}, error={}",peer_addr,local_addr,err);
                _ = this_conn.close().await;
            } else {
                log::info!(target:"switch","handle incoming stream ok, peer={}, local={}",peer_addr,local_addr);
            }
        });

        // start "/ipfs/id/1.0.0" handshake.
        self.identity_request(conn)
            .timeout(self.immutable.timeout)
            .await
            .ok_or(Error::Timeout)??;

        Ok(())
    }

    async fn handle_incoming_stream(&self, conn: &mut Connection) -> Result<()> {
        loop {
            let mut stream = conn.accept().await?;

            let (protoco_id, _) = listener_select_proto(&mut stream, &self.immutable.protos)
                .timeout(self.immutable.timeout)
                .await
                .ok_or(Error::Timeout)??;

            let peer_addr = conn.peer_addr()?;
            let local_addr = conn.local_addr()?;

            let this = self.clone();
            let protoco_id = protoco_id.clone();

            let mut this_conn = conn.try_clone()?;

            spawn_ok(async move {
                if let Err(err) = this
                    .dispatch_stream(protoco_id, stream, &mut this_conn)
                    .await
                {
                    log::error!(target:"switch","dispatch stream, peer={}, local={}, err={}",peer_addr,local_addr,err);
                    _ = this_conn.close();
                } else {
                    log::error!(target:"switch","dispatch stream ok, peer={}, local={}",peer_addr,local_addr);
                }
            })
        }
    }

    async fn dispatch_stream(
        &self,
        protoco_id: String,
        stream: Stream,
        conn: &mut Connection,
    ) -> Result<()> {
        let peer_addr = conn.peer_addr()?;
        let conn_peer_id = conn.public_key()?.to_peer_id();

        match protoco_id.as_str() {
            PROTOCOL_IPFS_ID => self.identity_response(&peer_addr, stream).await?,
            PROTOCOL_IPFS_PUSH_ID => self.identity_push(&conn_peer_id, stream).await?,
            PROTOCOL_IPFS_PING => self.ping_echo(stream).await?,
            _ => {
                self.immutable
                    .proto_handlers
                    .get(&protoco_id)
                    .expect("select_proto should work fine.")
                    .dispatch(&protoco_id, stream)
                    .await?;
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

            if self.immutable.max_identity_len < body_len {
                return Err(Error::IdentityOverflow(self.immutable.max_identity_len));
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
        log::info!(target:"switch","{} observed addrs: {:#?}", peer_id, observed_addrs);

        self.update_routes(peer_id, &raddrs).await
    }

    /// Start a "/ipfs/id/1.0.0" handshake.
    async fn identity_request(&self, conn: &mut Connection) -> Result<()> {
        let stream = conn.connect().await?;

        let conn_peer_id = conn.public_key()?.to_peer_id();

        self.identity_push(&conn_peer_id, stream).await
    }

    async fn identity_response(&self, peer_addr: &Multiaddr, mut stream: Stream) -> Result<()> {
        log::trace!("handle identity request");

        let mut identity = Identity::new();

        identity.set_observedAddr(peer_addr.to_vec());

        identity.set_publicKey(self.public_key().encode_protobuf());

        identity.set_agentVersion(self.immutable.agent_version.to_owned());

        identity.listenAddrs = self
            .immutable
            .laddrs
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
        if let Some(id) = self.immutable.route_table.peer_id_of(raddr).await? {
            if let Some(conn) = self.mutable.lock().unwrap().get_conn(&id) {
                return Ok((conn, self.clone()).into());
            }
        }

        self.connect_peer_to_prv(raddr).await
    }

    async fn connect_peer_to_prv(&self, raddr: &Multiaddr) -> Result<Connection> {
        let transport = self
            .immutable
            .get_transport_by_address(raddr)
            .ok_or(Error::UnspportMultiAddr(raddr.to_owned()))?;

        let mut conn = transport.connect(raddr, self.clone()).await?;

        self.setup_conn(&mut conn).await?;

        Ok(conn)
    }

    /// Create a new connection to peer by id.
    ///
    /// This function will first check for a local connection cache,
    /// and if there is one, it will directly return the cached connection
    async fn connect_peer(&self, id: &PeerId) -> Result<Connection> {
        if let Some(conn) = self.mutable.lock().unwrap().get_conn(id) {
            return Ok((conn, self.clone()).into());
        }

        let mut raddrs = self.immutable.route_table.get(id).await?;

        let mut last_error = None;

        use futures::TryStreamExt;

        while let Some(raddr) = raddrs.try_next().await? {
            match self.connect_peer_to_prv(&raddr).await {
                Ok(conn) => return Ok(conn),
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap_or(Error::ConnectPeer(id.to_owned())))
    }

    pub(crate) fn return_unused_conn(&self, conn: Box<dyn DriverConnection>) -> Result<()> {
        self.mutable.lock().unwrap().put_conn(conn)
    }
}

impl Switch {
    /// Create a new `Switch` builder.
    pub fn new<A, K, R>(agent_version: A, keystore: K, route_table: R) -> SwitchBuilder
    where
        A: AsRef<str>,
        K: DriverKeyStore + 'static,
        R: DriverRouteTable + 'static,
    {
        SwitchBuilder {
            ops: Ok(ImmutableSwitch::new(
                agent_version.as_ref().to_owned(),
                keystore.into(),
                route_table.into(),
            )),
        }
    }

    /// Update one `peer_id`'s route table.
    pub async fn update_routes(&self, peer_id: PeerId, addrs: &[Multiaddr]) -> Result<()> {
        Ok(self.immutable.route_table.put(peer_id, addrs).await?)
    }

    /// Delete a peer's route table.
    pub async fn delete_routes(&self, peer_id: &PeerId) -> Result<()> {
        Ok(self.immutable.route_table.delete(peer_id).await?)
    }

    /// Returns the public key of this switch.
    ///
    /// This returned value is provided by [`KeyStore`] service.
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Create a new stream to `peer_id` with provided `protos`.
    pub async fn connect<I>(&self, peer_id: &PeerId, protos: I) -> Result<(Stream, String)>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut conn = self.connect_peer(peer_id).await?;

        let mut stream = conn.connect().await?;

        let (protocol_id, _) = dialer_select_proto(&mut stream, protos, Version::V1)
            .timeout(self.immutable.timeout)
            .await
            .ok_or(Error::Timeout)??;

        Ok((stream, protocol_id.as_ref().to_owned()))
    }

    /// Create a new stream to `peer_id` with provided `protos`.
    pub async fn connect_to<I>(&self, raddr: &Multiaddr, protos: I) -> Result<(Stream, String)>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut conn = self.connect_peer_to(raddr).await?;

        let mut stream = conn.connect().await?;

        let (protocol_id, _) = dialer_select_proto(&mut stream, protos, Version::V1)
            .timeout(self.immutable.timeout)
            .await
            .ok_or(Error::Timeout)??;

        Ok((stream, protocol_id.as_ref().to_owned()))
    }
}
