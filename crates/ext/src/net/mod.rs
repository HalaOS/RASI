//! This module extends [`rasi::net`](https://docs.rs/rasi/latest/rasi/net/index.html)
//! to add support for additional protocols.
//!
//! For example [`quic`], [`UdpGroup`](udp_group), etc,.

pub mod quic;
pub mod udp_group;
