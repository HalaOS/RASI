//! Extend features or experimental features that are useful for asynchronous programming.
//!

#[cfg(feature = "batching")]
#[cfg_attr(docsrs, doc(cfg(feature = "batching")))]
pub mod batching;

#[cfg(feature = "event_map")]
#[cfg_attr(docsrs, doc(cfg(feature = "event_map")))]
pub mod event_map;

#[cfg(feature = "queue")]
#[cfg_attr(docsrs, doc(cfg(feature = "queue")))]
pub mod queue;
