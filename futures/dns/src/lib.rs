#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod client;

mod errors;
pub use errors::*;

#[cfg(feature = "rasi")]
#[cfg_attr(docsrs, doc(cfg(feature = "rasi")))]
pub mod rasi;
