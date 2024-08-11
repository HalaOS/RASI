//! Represents the libp2p protocol driver.
//!

use std::{collections::HashMap, ops::Deref, sync::Arc, time::Duration};

use futures::TryStreamExt;
use multiaddr::Multiaddr;
use rasi::task::spawn_ok;

use crate::{
    driver_wrapper, keystore::syscall::DriverKeyStore, routetable::syscall::DriverRouteTable,
    transport::syscall::DriverTransport, Result, Switch, SwitchBuilder,
};

/// A libp2p protocol driver must implement the `Driver-*` traits in this module.
pub mod syscall {
    use std::io::Result;

    use async_trait::async_trait;

    use crate::{transport::Stream, Switch};

    use super::ProtocolHandler;

    /// A protocol server side code should implement this trait.
    #[async_trait]
    pub trait DriverProtocol: Sync + Send {
        /// Returns protocol display name.
        fn protos(&self) -> &'static [&'static str];

        async fn create(&self, switch: &Switch) -> Result<ProtocolHandler>;
    }

    /// A dyn object created by [`create`](DriverProtocol::create) function.
    #[async_trait]
    pub trait DriverProtocolHandler: Sync + Send {
        /// Handle a new incoming stream.
        async fn dispatch(&self, negotiated: &str, stream: Stream) -> Result<()>;
    }
}

driver_wrapper!(
["A type wrapper of [`DriverProtocol`](syscall::DriverProtocol)"]
Protocol[syscall::DriverProtocol]
);

driver_wrapper!(
["A type wrapper of [`DriverProtocolHandler`](syscall::DriverProtocolHandler)"]
ProtocolHandler[syscall::DriverProtocolHandler]
);

/// A builder for ServeMux.
pub struct ServeMuxBuilder {
    protocols: Vec<Protocol>,
    switch_builder: SwitchBuilder,
}

impl ServeMuxBuilder {
    /// Add one `protocol` support.
    pub fn handle<P>(mut self, protocol: P) -> Self
    where
        P: syscall::DriverProtocol + 'static,
    {
        self = self.protos(protocol.protos());

        self.protocols.push(protocol.into());

        self
    }

    /// Set the `max_incoming_queue_size`, the default value is `200`
    pub fn max_incoming_queue_size(mut self, value: usize) -> Self {
        self.switch_builder = self.switch_builder.max_identity_packet_size(value);

        self
    }
    /// Replace default [`MemoryKeyStore`].
    pub fn keystore<K>(mut self, value: K) -> Self
    where
        K: DriverKeyStore + 'static,
    {
        self.switch_builder = self.switch_builder.keystore(value);

        self
    }

    /// Replace default [`MemoryRouteTable`].
    pub fn route_table<R>(mut self, value: R) -> Self
    where
        R: DriverRouteTable + 'static,
    {
        self.switch_builder = self.switch_builder.route_table(value);

        self
    }

    /// Set the protocol timeout, the default value is `10s`
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.switch_builder = self.switch_builder.timeout(duration);

        self
    }

    /// Set the receive max buffer length of identity protocol.
    pub fn max_identity_packet_size(mut self, value: usize) -> Self {
        self.switch_builder = self.switch_builder.max_identity_packet_size(value);

        self
    }

    /// Register a new transport driver for the switch.
    pub fn transport<T>(mut self, value: T) -> Self
    where
        T: DriverTransport + 'static,
    {
        self.switch_builder = self.switch_builder.transport(value);

        self
    }

    /// Add a new listener which is bound to `laddr`.
    pub fn bind(mut self, laddr: Multiaddr) -> Self {
        self.switch_builder = self.switch_builder.bind(laddr);

        self
    }

    /// Set the protocol list of this switch accepts.
    pub fn protos<I>(mut self, value: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.switch_builder = self.switch_builder.protos(value);

        self
    }

    /// Consume self and create a new [`ServeMux`] instance.
    pub async fn create(self) -> Result<ServeMux> {
        let switch = self.switch_builder.create().await?;

        let mut handlers = HashMap::new();

        for proto in self.protocols {
            let handler = Arc::new(proto.create(&switch).await?);

            for id in proto.protos() {
                handlers.insert(id.to_string(), handler.clone());
            }
        }

        let mux = ServeMux {
            handlers: Arc::new(handlers),
            switch,
        };

        spawn_ok(mux.clone().dispatch());

        Ok(mux)
    }
}

/// `ServeMux` is a libp2p request mulitplexer.
#[derive(Clone)]
pub struct ServeMux {
    handlers: Arc<HashMap<String, Arc<ProtocolHandler>>>,
    switch: Switch,
}

impl From<ServeMux> for Switch {
    fn from(value: ServeMux) -> Self {
        value.switch
    }
}

impl Deref for ServeMux {
    type Target = Switch;
    fn deref(&self) -> &Self::Target {
        &self.switch
    }
}

impl ServeMux {
    /// Create a builder to construct the `ServeMux`
    pub fn new<A>(agent_version: A) -> ServeMuxBuilder
    where
        A: AsRef<str>,
    {
        ServeMuxBuilder {
            protocols: Default::default(),
            switch_builder: Switch::new(agent_version),
        }
    }

    async fn dispatch(self) {
        if let Err(err) = self.dispatch_prv().await {
            log::error!(target:"ServerMux", "background dispatch, err={}",err);
        }
    }

    async fn dispatch_prv(self) -> Result<()> {
        let mut incoming = self.switch.into_incoming();

        while let Some((stream, negotiated)) = incoming.try_next().await? {
            let id = stream.id().to_string();

            log::info!(target:"ServerMux", "accept new stream, id={}, negotiated={}", id, negotiated);

            if let Some(handler) = self.handlers.get(&negotiated) {
                let handler = handler.clone();

                spawn_ok(async move {
                    if let Err(err) = handler.dispatch(&negotiated, stream).await {
                        log::info!(
                            target:"ServerMux",
                            "dispatch stream, id={}, negotiated={}, err={}",
                            id,
                            negotiated,
                            err
                        );
                    } else {
                        log::info!(target:"ServerMux", "dispatch stream ok, id={}, negotiated={}", id, negotiated,);
                    }
                })
            } else {
                log::warn!(
                    target:"ServerMux",
                    "can't dispatch stream, id={}, negotiated={},",
                    id,
                    negotiated
                );
            }
        }

        Ok(())
    }
}
