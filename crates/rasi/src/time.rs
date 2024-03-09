//! Utilities for tracking time.
//!

use std::{
    io,
    task::Poll,
    time::{Duration, Instant},
};

use futures::{Future, FutureExt};
use rasi_syscall::{global_timer, Handle};

struct Sleep {
    deadline: Instant,
    handle: Option<Handle>,
    syscall: &'static dyn rasi_syscall::Timer,
}

impl Sleep {
    /// Create timer future with custom [`syscall`](rasi_syscall::Timer)
    fn new_with(deadline: Instant, syscall: &'static dyn rasi_syscall::Timer) -> Self {
        Self {
            deadline,
            handle: None,
            syscall,
        }
    }
}

impl Future for Sleep {
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
    let timer = Sleep::new_with(Instant::now() + duraton, syscall);

    timer.await.expect("Call register_global_timer first");
}

async fn timeout_at_with(at: Instant, syscall: &'static dyn rasi_syscall::Timer) {
    let timer = Sleep::new_with(at, syscall);

    timer.await.expect("Call register_global_timer first");
}

/// Extension trait for [`Future`].
///
/// This trait brings a timeout capability to any object that implements the [`Future`] trait.
pub trait TimeoutExt: Future {
    /// Awaits a future or times out after a duration of time.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use std::time::Duration;
    ///
    /// use rasi::prelude::*;
    ///
    /// let never = futures::future::pending::<()>();
    /// let dur = Duration::from_millis(5);
    /// assert!(never.timeout(dur).await.is_none());
    /// #
    /// # Ok(()) }) }
    /// ```

    fn timeout(self, duration: Duration) -> impl Future<Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        self.timeout_with(duration, global_timer())
    }

    /// Using custom [`syscall`](rasi_syscall::Timer) to await a future or times out after a duration of time.
    ///
    /// see [`timeout`](TimeoutExt::timeout) for more information.
    fn timeout_with(
        self,
        duration: Duration,
        syscall: &'static dyn rasi_syscall::Timer,
    ) -> impl Future<Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        async move {
            futures::select! {
                _ = sleep_with(duration,syscall).fuse() => {
                    None
                }
                fut = self.fuse() => {
                    Some(fut)
                }

            }
        }
    }

    /// Awaits a future or times out at a time point after now.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use std::time::{Duration, Instant};
    ///
    /// use rasi::prelude::*;
    ///
    /// let never = futures::future::pending::<()>();
    /// let dur = Duration::from_millis(5);
    /// assert!(never.timeout_at(Instant::now() + dur).await.is_none());
    /// #
    /// # Ok(()) }) }
    /// ```

    fn timeout_at(self, at: Instant) -> impl Future<Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        self.timeout_at_with(at, global_timer())
    }

    /// Using custom [`syscall`](rasi_syscall::Timer) to await a future or times out at a time point after now.
    ///
    /// see [`timeout`](TimeoutExt::timeout_at) for more information.
    fn timeout_at_with(
        self,
        at: Instant,
        syscall: &'static dyn rasi_syscall::Timer,
    ) -> impl Future<Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        async move {
            futures::select! {
                fut = self.fuse() => {
                    Some(fut)
                }
                _ = timeout_at_with(at,syscall).fuse() => {
                    None
                }
            }
        }
    }
}

impl<T> TimeoutExt for T where T: Future {}
