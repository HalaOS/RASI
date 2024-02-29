//! cancelable syscall apis.

use crate::handle::Handle;

/// The returns type of [`WouldBlock`](std::io::ErrorKind::WouldBlock) operations.
pub enum CancelablePoll<T> {
    /// Operation is ready, returns operation result.
    Success(T),
    /// Operation is pending, returns operation cancel handle.
    Pending(Handle),
}

impl<T> CancelablePoll<T> {
    /// Cancel the pending io, if self == [`CancelablePoll::Pending`] then this function will not perform any action.
    pub fn cancel<SysCall: Cancelable>(&self, syscall: SysCall) {
        match self {
            CancelablePoll::Success(_) => {}
            CancelablePoll::Pending(handle) => syscall.cancel(handle),
        }
    }
}

/// Syscall to support cancel pending operation.
pub trait Cancelable {
    /// Cancel one pending operation.
    ///
    /// You can get the pending operation handle by [`CancelablePoll::Pending`]
    fn cancel(&self, handle: &Handle);
}
