//! The module provides various implementations of ethereum signer

mod signer;
pub use signer::*;

#[cfg(feature = "wallet")]
#[cfg_attr(docsrs, doc(cfg(feature = "wallet")))]
pub mod wallet;
