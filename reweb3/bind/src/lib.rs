#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod binder;
pub mod mapping;
pub mod typedef;

#[cfg(feature = "rustgen")]
#[cfg_attr(docsrs, doc(cfg(feature = "rustgen")))]
pub mod rustgen;
