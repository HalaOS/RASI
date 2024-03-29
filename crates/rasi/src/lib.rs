#![cfg_attr(docsrs, feature(doc_cfg))]

pub use rasi_syscall as syscall;

#[cfg(feature = "executor")]
#[cfg_attr(docsrs, doc(cfg(feature = "executor")))]
pub mod executor;
#[cfg(feature = "fs")]
#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
pub mod fs;
#[cfg(feature = "net")]
#[cfg_attr(docsrs, doc(cfg(feature = "net")))]
pub mod net;
#[cfg(feature = "time")]
#[cfg_attr(docsrs, doc(cfg(feature = "time")))]
pub mod time;

#[cfg(feature = "inter_process")]
#[cfg_attr(docsrs, doc(cfg(feature = "inter_process")))]
pub mod inter_process;

pub mod utils;

pub mod prelude;

#[cfg(feature = "fs")]
#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
pub use rasi_syscall::path;

pub use futures::{channel, future, io, sink, stream};
