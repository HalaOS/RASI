//! syscall for timeout.

use std::{
    io,
    sync::OnceLock,
    task::{Poll, Waker},
    time::Instant,
};

use crate::handle::Handle;

/// Timer-related system call interface
pub trait Timer: Send + Sync {
    /// Create new `deadline` timer, returns [`None`] if the `deadline` instant is reached.
    fn deadline(&self, deadline: Instant) -> io::Result<Option<Handle>>;

    /// Wait timeout event.
    ///
    /// Returns [`Poll::Ready(())`](Poll::Ready) if the timer already reached the deadline,
    /// otherwise returns [`Poll::Pending`] and needs to be retried later.
    ///
    /// We don't need to return [`CancelablePoll`](crate::CancelablePoll),
    /// because the implementation should stop the timer when the timer handler drops.
    fn timeout_wait(&self, waker: Waker, handle: &Handle) -> Poll<()>;
}

static GLOBAL_TIMER: OnceLock<Box<dyn Timer>> = OnceLock::new();

/// Register provided [`Timer`] as global timer implementation.
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_global_timer<E: Timer + 'static>(executor: E) {
    if GLOBAL_TIMER.set(Box::new(executor)).is_err() {
        panic!("Multiple calls to register_global_timer are not permitted!!!");
    }
}

/// Get global register [`Timer`] syscall interface.
///
/// # Panic
///
/// You should call [`register_global_timer`] first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_timer first`
pub fn global_timer() -> &'static dyn Timer {
    GLOBAL_TIMER
        .get()
        .expect("Call register_global_timer first")
        .as_ref()
}
