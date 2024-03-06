//! Utility to batch poll a set of [`udp sockets`](rasi::net::UdpSocket)
//!
//! UdpGroup internally uses `batching::Group` to drive the batch polling. [*Read more*](crate::future::batching::Group)
//!
//! # Example
