#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "abi")]
#[cfg_attr(docsrs, doc(cfg(feature = "abi")))]
pub mod abi;

pub mod eip;

pub mod errors;

pub mod primitives;

#[cfg(feature = "signer")]
#[cfg_attr(docsrs, doc(cfg(feature = "signer")))]
pub mod signer;

#[cfg(feature = "clients")]
#[cfg_attr(docsrs, doc(cfg(feature = "clients")))]
pub mod clients;

#[cfg(feature = "rlp")]
#[cfg_attr(docsrs, doc(cfg(feature = "rlp")))]
pub mod rlp;

#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub mod macros;

pub use primitives::{balance::TransferOptions, *};

#[cfg(feature = "abi")]
pub use abi::{from_abi, to_abi};

pub use serde;

#[cfg(feature = "signer")]
pub use signer::SignerWithProvider;

#[cfg(feature = "clients")]
pub use clients::Client;
