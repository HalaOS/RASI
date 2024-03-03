//! Utilities for tracking time.
//!

use std::{
    io,
    task::Poll,
    time::{Duration, Instant},
};

use futures::Future;
use rasi_syscall::Handle;

struct Timer {
    deadline: Instant,
    handle: Option<Handle>,
}

impl Timer {
    fn new(deadline: Instant) -> Self {
        Self {
            deadline,
            handle: None,
        }
    }
}

impl Future for Timer {
    type Output = io::Result<()>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.handle.is_none() {
            let handle =
                rasi_syscall::global_timer().deadline(cx.waker().clone(), self.deadline)?;

            if handle.is_none() {
                return Poll::Ready(Ok(()));
            }

            self.handle = handle;

            return Poll::Pending;
        }

        rasi_syscall::global_timer()
            .timeout_wait(cx.waker().clone(), self.handle.as_ref().unwrap())
            .map(|_| Ok(()))
    }
}

/// Create a new timer and block the calling task until the "deadline" is reached.
///
/// # Panic
///
/// You should call [`register_global_timer`](rasi_syscall::register_global_timer) first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_timer first`
pub async fn timeout_at(deadline: Instant) {
    let timer = Timer::new(deadline);

    timer.await.expect("Call register_global_timer first");
}

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// # Panic
///
/// You should call [`register_global_timer`](rasi_syscall::register_global_timer) first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_timer first`
pub async fn sleep(duraton: Duration) {
    timeout_at(Instant::now() + duraton).await
}
