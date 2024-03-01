#![cfg_attr(docsrs, feature(doc_cfg))]

mod cancellable;
mod handle;
pub use cancellable::*;
pub use handle::*;

#[cfg(feature = "executor")]
mod executor;

#[cfg(feature = "fs")]
mod fs;

#[cfg(feature = "net")]
mod net;

#[cfg(feature = "time")]
mod time;

#[cfg(feature = "executor")]
#[cfg_attr(docsrs, doc(cfg(feature = "executor")))]
pub use executor::*;

#[cfg(feature = "fs")]
#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
pub use fs::*;

#[cfg(feature = "net")]
#[cfg_attr(docsrs, doc(cfg(feature = "net")))]
pub use net::*;

#[cfg(feature = "time")]
#[cfg_attr(docsrs, doc(cfg(feature = "time")))]
pub use time::*;
