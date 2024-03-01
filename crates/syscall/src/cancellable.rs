//! cancelable syscall apis.

use crate::handle::Handle;

/// The returns type of [`WouldBlock`](std::io::ErrorKind::WouldBlock) operations.
pub enum CancelablePoll<T> {
    /// Operation is ready, returns operation result.
    Success(T),
    /// Operation is pending, returns operation cancel handle.
    ///
    /// When pending handle drops, the syscall implementation should cancel
    /// the pending operation referenced by this handle.
    Pending(Handle),
}
