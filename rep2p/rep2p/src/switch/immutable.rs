/// immutable context data for one switch.
struct ImmutableSwitch {
    /// The maximun size of active connections pool size.
    max_conn_pool_size: usize,
    /// The maximun length of inbound streams queue.
    max_inbound_length: usize,
    /// The value of rpc timeout.
    timeout: Duration,
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
            max_identity_packet_size: 4096,
            transports: vec![],
            keystore: MemoryKeyStore::random().into(),
            peer_book: MemoryPeerBook::default().into(),
        }
    }

    fn get_transport_by_address(&self, laddr: &Multiaddr) -> Option<&Transport> {
        self.transports
            .iter()
            .find(|transport| transport.multiaddr_hit(laddr))
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
