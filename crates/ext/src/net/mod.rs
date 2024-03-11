//! This module extends [`rasi::net`](https://docs.rs/rasi/latest/rasi/net/index.html)
//! to add support for additional protocols.
//!
//! For example [`QuicConn`], [`UdpGroup`], etc,.

#[cfg(feature = "quic")]
#[cfg_attr(docsrs, doc(cfg(feature = "quic")))]
pub mod quic;

#[cfg(feature = "udp_group")]
#[cfg_attr(docsrs, doc(cfg(feature = "udp_group")))]
pub mod udp_group;

#[cfg(feature = "tls")]
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub mod tls;
