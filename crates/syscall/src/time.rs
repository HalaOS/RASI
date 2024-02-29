//! syscall for timeout.

use std::{
    io,
    task::{Poll, Waker},
    time::Instant,
};

use crate::handle::Handle;

/// Timer-related system call interface
pub trait Timer {
    /// Create new `deadline` timer, returns [`None`] if the `deadline` instant is reached.
    fn deadline(&self, deadline: Instant) -> io::Result<Option<Handle>>;

    /// Wait timeout event.
    ///
    /// Returns [`Poll::Ready(())`](Poll::Ready) if the timer already reached the deadline,
    /// otherwise returns [`Poll::Pending`] and needs to be retried later.
    fn timeout_wait(&self, waker: Waker, handle: &Handle) -> Poll<()>;
}
