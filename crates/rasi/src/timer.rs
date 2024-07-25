//! Future-based utilities for tracking time.
//!

use std::{
    io::Result,
    sync::OnceLock,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{Future, FutureExt};

/// A timer driver must implement the Driver-* traits in this module.
pub mod syscall {
    use super::*;

    /// The main entry of tracking time.
    pub trait Driver: Sync + Send {
        /// Create new `deadline` timer, returns [`None`] if the `deadline` instant is reached.
        fn deadline(&self, deadline: Instant) -> Result<Option<DeadLine>>;
    }

    /// Driver-specific `DeadLine` object.
    pub trait DriverDeadLine: Sync + Send {
        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<()>;
    }
}

/// Future for deadline event.
pub struct DeadLine(Box<dyn syscall::DriverDeadLine>);

impl<D: syscall::DriverDeadLine + 'static> From<D> for DeadLine {
    fn from(value: D) -> Self {
        Self(Box::new(value))
    }
}

impl DeadLine {
    /// Returns inner driver-specific implementation.
    pub fn as_raw_ptr(&self) -> &dyn syscall::DriverDeadLine {
        &*self.0
    }
}

impl Future for DeadLine {
    type Output = ();
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.as_raw_ptr().poll_ready(cx)
    }
}

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// # Panic
///
/// You should call [`register_timer_driver`] first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_timer first`
pub async fn sleep(duration: Duration) {
    sleep_with(duration, get_timer_driver()).await
}

/// Sleep current thread until provided at.
pub async fn sleep_until(at: Instant) {
    sleep_until_with(at, get_timer_driver()).await
}

pub async fn sleep_until_with(at: Instant, driver: &dyn syscall::Driver) {
    if let Some(dead_line) = driver
        .deadline(at)
        .expect("Call register_global_timer first")
    {
        dead_line.await
    }
}

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// # Panic
///
/// You should call [`register_timer_driver`] first to register implementation,
/// otherwise this function will cause a panic with `Call register_timer_driver first`
pub async fn sleep_with(duration: Duration, driver: &dyn syscall::Driver) {
    if let Some(dead_line) = driver
        .deadline(Instant::now() + duration)
        .expect("Call register_global_timer first")
    {
        dead_line.await
    }
}

/// Extension trait for [`Future`].
///
/// This trait brings a timeout capability to any object that implements the [`Future`] trait.
pub trait TimeoutExt: Future {
    /// Awaits a future or times out after a duration of time.
    ///
    fn timeout(self, duration: Duration) -> impl Future<Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        self.timeout_with(duration, get_timer_driver())
    }

    fn timeout_with(
        self,
        duration: Duration,
        driver: &dyn syscall::Driver,
    ) -> impl Future<Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        async move {
            futures::select! {
                _ =  sleep_with(duration,driver).fuse() => {
                    None
                }
                fut = self.fuse() => {
                    Some(fut)
                }

            }
        }
    }

    /// Awaits a future or times out after a duration of time.
    ///
    fn timeout_at(self, at: Instant) -> impl Future<Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        self.timeout_at_with(at, get_timer_driver())
    }

    fn timeout_at_with(
        self,
        at: Instant,
        driver: &dyn syscall::Driver,
    ) -> impl Future<Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        async move {
            futures::select! {
                _ =  sleep_until_with(at,driver).fuse() => {
                    None
                }
                fut = self.fuse() => {
                    Some(fut)
                }

            }
        }
    }
}

impl<T> TimeoutExt for T where T: Future {}

static GLOBAL_TIMER: OnceLock<Box<dyn syscall::Driver>> = OnceLock::new();

/// Register provided [`syscall::Driver`] as global timer implementation.
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_timer_driver<T: syscall::Driver + 'static>(timer: T) {
    if GLOBAL_TIMER.set(Box::new(timer)).is_err() {
        panic!("Multiple calls to register_timer_driver are not permitted!!!");
    }
}

/// Get the [`syscall::Driver`] from global context of this application.
///
/// # Panic
///
/// You should call [`register_timer_driver`] first to register implementation,
/// otherwise this function will cause a panic with `Call register_timer_driver first`
pub fn get_timer_driver() -> &'static dyn syscall::Driver {
    GLOBAL_TIMER
        .get()
        .expect("Call register_global_timer first")
        .as_ref()
}
