use std::{
    io::Result,
    sync::OnceLock,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{Future, FutureExt};

pub trait TimerDriver: Sync + Send {
    /// Create new `deadline` timer, returns [`None`] if the `deadline` instant is reached.
    fn deadline(&self, deadline: Instant) -> Result<Option<DeadLine>>;
}

/// Driver-specific `DeadLine` object.
pub trait TDDeadLine: Sync + Send {
    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<()>;
}

/// Future for deadline event.
pub struct DeadLine(Box<dyn TDDeadLine>);

impl<D: TDDeadLine + 'static> From<D> for DeadLine {
    fn from(value: D) -> Self {
        Self(Box::new(value))
    }
}

impl DeadLine {
    /// Returns inner driver-specific implementation.
    pub fn as_raw_ptr(&self) -> &dyn TDDeadLine {
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
/// You should call [`register_global_timer`](rasi_syscall::register_global_timer) first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_timer first`
pub async fn sleep(duration: Duration) {
    sleep_with(duration, get_timer_driver()).await
}

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// # Panic
///
/// You should call [`register_global_timer`](rasi_syscall::register_global_timer) first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_timer first`
pub async fn sleep_with(duration: Duration, driver: &dyn TimerDriver) {
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
        driver: &dyn TimerDriver,
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
}

impl<T> TimeoutExt for T where T: Future {}

static GLOBAL_TIMER: OnceLock<Box<dyn TimerDriver>> = OnceLock::new();

/// Register provided [`Timer`] as global timer implementation.
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_timer_driver<T: TimerDriver + 'static>(timer: T) {
    if GLOBAL_TIMER.set(Box::new(timer)).is_err() {
        panic!("Multiple calls to register_timer_driver are not permitted!!!");
    }
}

/// Get the globally registered instance of [`Timer`].
///
/// # Panic
///
/// You should call [`register_timer_driver`] first to register implementation,
/// otherwise this function will cause a panic with `Call register_timer_driver first`
pub fn get_timer_driver() -> &'static dyn TimerDriver {
    GLOBAL_TIMER
        .get()
        .expect("Call register_global_timer first")
        .as_ref()
}
