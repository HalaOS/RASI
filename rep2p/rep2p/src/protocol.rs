//! Represents the libp2p protocol driver.
//!
use crate::driver_wrapper;

/// A libp2p protocol driver must implement the `Driver-*` traits in this module.
pub mod syscall {
    use std::io::Result;

    use async_trait::async_trait;

    use crate::{transport::Stream, Switch};

    use super::ProtocolHandler;

    /// A protocol server side code should implement this trait.
    pub trait DriverProtocol: Sync + Send {
        /// Returns protocol display name.
        fn name(&self) -> &str;

        fn create(&self) -> Result<ProtocolHandler>;
    }

    /// A dyn object created by [`create`](DriverProtocol::create) function.
    #[async_trait]
    pub trait DriverProtocolHandler: Sync + Send {
        /// Process protocol startup works.
        async fn start(&self, switch: &Switch) -> Result<()>;
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
