//! Utilities types / functions.

use std::{
    io,
    task::{Context, Poll},
};

use futures::Future;
use rasi_syscall::{CancelablePoll, Handle};

/// Future object created by function [`cancelable_would_block`]
pub struct CancelableWouldBlock<F> {
    f: F,
    cancel_handle: Option<Handle>,
}

impl<F, R> Future for CancelableWouldBlock<F>
where
    F: FnMut(&mut Context<'_>) -> CancelablePoll<io::Result<R>> + Unpin,
{
    type Output = io::Result<R>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match (self.f)(cx) {
            CancelablePoll::Ready(v) => Poll::Ready(v),
            CancelablePoll::Pending(cancel_handle) => {
                self.cancel_handle = cancel_handle;

                Poll::Pending
            }
        }
    }
}

/// Create a new future from `F: FnMut(&mut Context<'_>) -> CancelablePoll<io::Result<R>> + Unpin,`
pub fn cancelable_would_block<F, R>(f: F) -> CancelableWouldBlock<F>
where
    F: FnMut(&mut Context<'_>) -> CancelablePoll<io::Result<R>> + Unpin,
{
    CancelableWouldBlock {
        f,
        cancel_handle: None,
    }
}
