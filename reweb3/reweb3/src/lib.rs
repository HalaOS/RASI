#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "abi")]
#[cfg_attr(docsrs, doc(cfg(feature = "abi")))]
pub mod abi;

#[cfg(feature = "abi-json")]
#[cfg_attr(docsrs, doc(cfg(feature = "abi-json")))]
pub mod abi_json;

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
