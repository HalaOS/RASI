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

#[cfg(feature = "providers")]
#[cfg_attr(docsrs, doc(cfg(feature = "providers")))]
pub mod providers;

#[cfg(feature = "rlp")]
#[cfg_attr(docsrs, doc(cfg(feature = "rlp")))]
pub mod rlp;
