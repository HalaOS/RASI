mod api;
pub use api::*;
/// [`AsyncLockable`] type maker
pub mod maker;

mod spin;

#[cfg(not(feature = "sync_parking_lot"))]
pub use spin::*;

/// Pure userspace spin locker implementation.
pub mod spin_simple {
    pub use super::spin::*;
}

#[cfg(feature = "sync_parking_lot")]
mod parking_lot;
#[cfg(feature = "sync_parking_lot")]
pub use parking_lot::*;
