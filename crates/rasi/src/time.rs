//! Utilities for tracking time.
//!

use std::{
    io,
    time::{Duration, Instant},
};

use futures::Future;
use rasi_syscall::Handle;

struct Timer(Handle);

impl Timer {
    fn new(deadline: Instant) -> io::Result<Option<Self>> {
        rasi_syscall::global_timer()
            .deadline(deadline)
            .map(|h| h.map(|h| Self(h)))
    }
}

impl Future for Timer {
    type Output = ();
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        rasi_syscall::global_timer().timeout_wait(cx.waker().clone(), &self.0)
    }
}

/// Create a new timer and block the calling task until the "deadline" is reached.
///
/// # Panic
///
/// You should call [`register_global_timer`](rasi_syscall::register_global_timer) first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_timer first`
pub async fn timeout_at(deadline: Instant) {
    if let Some(timer) = Timer::new(deadline).expect("Call register_global_timer first") {
        timer.await
    }
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
