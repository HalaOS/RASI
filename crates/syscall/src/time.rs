//! syscall for timeout.

use std::{
    io,
    task::{Poll, Waker},
    time::Instant,
};

use crate::handle::Handle;

/// timeout-related system call interface
pub trait Timeout {
    /// Create new `deadline` timer, returns [`None`] if the `deadline` instant is reached.
    fn deadline(&self, deadline: Instant) -> io::Result<Option<Handle>>;

    /// Wait timeout event.
    ///
    /// Returns [`Poll::Ready(())`] if the timer already reached the deadline,
    /// otherwise returns [`Poll::Pending`].
    fn timeout_wait(&self, waker: Waker, handle: &Handle) -> Poll<()>;
}
