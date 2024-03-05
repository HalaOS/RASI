#![cfg_attr(docsrs, feature(doc_cfg))]

mod cancellable;
mod handle;
pub use cancellable::*;
pub use handle::*;

#[cfg(any(feature = "executor", docsrs))]
#[cfg_attr(docsrs, doc(cfg(feature = "executor")))]
mod executor;

#[cfg(any(feature = "fs", docsrs))]
#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
mod fs;

#[cfg(any(feature = "net", docsrs))]
#[cfg_attr(docsrs, doc(cfg(feature = "net")))]
mod net;

#[cfg(any(feature = "time", docsrs))]
#[cfg_attr(docsrs, doc(cfg(feature = "time")))]
mod time;

#[cfg(any(feature = "executor", docsrs))]
pub use executor::*;

#[cfg(any(feature = "fs", docsrs))]
pub use fs::*;

#[cfg(any(feature = "net", docsrs))]
pub use net::*;

#[cfg(any(feature = "time", docsrs))]
pub use time::*;

pub mod path;
