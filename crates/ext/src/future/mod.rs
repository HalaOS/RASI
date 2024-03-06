//! useful extensions of `future`.

#[cfg(feature = "batching")]
#[cfg_attr(docsrs, doc(cfg(feature = "batching")))]
pub mod batching;

#[cfg(feature = "event_map")]
#[cfg_attr(docsrs, doc(cfg(feature = "event_map")))]
pub mod event_map;
