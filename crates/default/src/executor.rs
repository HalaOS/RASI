//! The implementation of [`Executor`] syscall.
//!
//!

use std::io;

use futures::{executor::ThreadPool, future::BoxFuture};
use rasi_syscall::{register_global_executor, Executor};

/// The implementation of [`Executor`] syscall.
///
/// This type using [`ThreadPool`] as inner task executor.
pub struct FuturesExecutor {
    thread_pool: ThreadPool,
}

impl FuturesExecutor {
    pub fn new(pool_size: usize) -> io::Result<Self> {
        Ok(Self {
            thread_pool: ThreadPool::builder().pool_size(pool_size).create()?,
        })
    }
}

impl Executor for FuturesExecutor {
    fn spawn_boxed(&self, fut: BoxFuture<'static, ()>) {
        self.thread_pool.spawn_ok(fut)
    }
}

/// Create and register [`ThreadPool`] as the `rasi` global [`Executor`] syscall.
/// using parameter `pool_size` to specify the [`ThreadPool`] size.
///
/// You may not call this function twice, otherwise will cause a panic. [`read more`](`register_global_executor`)
pub fn register_futures_executor_with_pool_size(pool_size: usize) {
    register_global_executor(
        FuturesExecutor::new(pool_size).expect("Create `FuturesExecutor` error."),
    )
}

/// Create and register [`ThreadPool`] as the `rasi` global [`Executor`] syscall,
/// the inner using [`num_cpus::get_physical()`] to specify the pool size.
///
/// See [`register_futures_executor_with_pool_size`] for more informations.
pub fn register_futures_executor() {
    register_futures_executor_with_pool_size(num_cpus::get_physical())
}
