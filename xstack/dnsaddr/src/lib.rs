use std::io::Result;

use async_trait::async_trait;
use xstack::multiaddr::Multiaddr;
use xstack::transport::syscall::DriverTransport;
use xstack::transport::{Listener, TransportConnection};
use xstack::Switch;

/// The `dnsaddr` transport implementation.
pub struct DnsAddr;

#[allow(unused)]
#[async_trait]
impl DriverTransport for DnsAddr {
    /// Create a server-side socket with provided [`laddr`](Multiaddr).
    async fn bind(&self, laddr: &Multiaddr, switch: Switch) -> Result<Listener> {
        todo!()
    }

    /// Connect to peer with remote peer [`raddr`](Multiaddr).
    async fn connect(&self, raddr: &Multiaddr, switch: Switch) -> Result<TransportConnection> {
        todo!()
    }

    /// Check if this transport support the protocol stack represented by the `addr`.
    fn multiaddr_hit(&self, addr: &Multiaddr) -> bool {
        todo!()
    }
}
