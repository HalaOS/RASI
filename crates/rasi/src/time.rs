//! Utilities for tracking time.
//!

use std::{
    io,
    task::Poll,
    time::{Duration, Instant},
};

use futures::Future;
use rasi_syscall::{global_timer, Handle};

struct Timer {
    deadline: Instant,
    handle: Option<Handle>,
    syscall: &'static dyn rasi_syscall::Timer,
}

impl Timer {
    /// Create timer future with custom [`syscall`](rasi_syscall::Timer)
    fn new_with(deadline: Instant, syscall: &'static dyn rasi_syscall::Timer) -> Self {
        Self {
            deadline,
            handle: None,
            syscall,
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
            let handle = self.syscall.deadline(cx.waker().clone(), self.deadline)?;

            if handle.is_none() {
                return Poll::Ready(Ok(()));
            }

            self.handle = handle;

            return Poll::Pending;
        }

        self.syscall
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
    timeout_at_with(deadline, global_timer()).await
}

/// Call `timeout_at` with custom [`syscall`](rasi_syscall::Timer), [`read more`](timeout_at)
pub async fn timeout_at_with(deadline: Instant, syscall: &'static dyn rasi_syscall::Timer) {
    let timer = Timer::new_with(deadline, syscall);

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
    sleep_with(duraton, global_timer()).await
}

/// Call `sleep` with custom [`syscall`](rasi_syscall::Timer), [`read more`](sleep)
pub async fn sleep_with(duraton: Duration, syscall: &'static dyn rasi_syscall::Timer) {
    timeout_at_with(Instant::now() + duraton, syscall).await
}
