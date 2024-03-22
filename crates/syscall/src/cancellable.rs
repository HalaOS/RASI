//! cancelable syscall apis.

use std::task::Poll;

use crate::handle::Handle;

/// The returns type of [`WouldBlock`](std::io::ErrorKind::WouldBlock) operations.
pub enum CancelablePoll<T> {
    /// Operation is ready, returns operation result.
    Ready(T),
    /// Operation is pending, returns operation cancel handle.
    ///
    /// When pending handle drops, the syscall implementation should cancel
    /// the pending operation referenced by this handle.
    Pending(Option<Handle>),
}

impl<T> CancelablePoll<T> {
    /// Returns `true` if the poll is a [`Ready`](CancelablePoll::Ready) value.
    pub const fn is_ready(&self) -> bool {
        if let CancelablePoll::Ready(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns `true` if the poll is a [`Pending`](CancelablePoll::Pending) value.
    pub const fn is_pending(&self) -> bool {
        !self.is_ready()
    }

    /// Maps a `CancelablePoll<T>` to `CancelablePoll<U>` by applying a function to a contained value.
    ///
    pub fn map<U, F>(self, f: F) -> CancelablePoll<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            CancelablePoll::Ready(t) => CancelablePoll::Ready(f(t)),
            CancelablePoll::Pending(h) => CancelablePoll::Pending(h),
        }
    }

    /// Split `CancelablePoll` into [`Poll`] and [`Option<Handle>`]
    pub fn into_poll(self) -> (Poll<T>, Option<Handle>) {
        match self {
            CancelablePoll::Ready(r) => (Poll::Ready(r), None),
            CancelablePoll::Pending(pending) => (Poll::Pending, pending),
        }
    }
}

/// Create a ready status of [`CancelablePoll`] from `F: FnOnce() -> T`.
pub fn ready<T, F>(f: F) -> CancelablePoll<T>
where
    F: FnOnce() -> T,
{
    CancelablePoll::Ready(f())
}

impl<T> From<Poll<T>> for CancelablePoll<T> {
    fn from(value: Poll<T>) -> Self {
        match value {
            Poll::Ready(r) => CancelablePoll::Ready(r),
            Poll::Pending => CancelablePoll::Pending(None),
        }
    }
}
