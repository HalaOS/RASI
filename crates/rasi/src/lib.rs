#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
pub mod fs;
#[cfg_attr(docsrs, doc(cfg(feature = "ipc")))]
pub mod ipc;
#[cfg_attr(docsrs, doc(cfg(feature = "net")))]
pub mod net;
#[cfg_attr(docsrs, doc(cfg(feature = "rdbc")))]
pub mod rdbc;
#[cfg_attr(docsrs, doc(cfg(feature = "task")))]
pub mod task;
#[cfg_attr(docsrs, doc(cfg(feature = "timer")))]
pub mod timer;
