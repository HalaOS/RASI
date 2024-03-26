//! Types and traits for working with asynchronous tasks.
//!
//! This module is similar to std::thread, except it uses asynchronous tasks in place of threads.

use futures::Future;
use rasi_syscall::{global_executor, syscall_block_on, syscall_spawn};

/// Use global register [syscall](rasi_syscall::Executor) to spawn a task
/// that polls the given future with output () to completion.
#[inline]
pub fn spawn<Fut>(fut: Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    syscall_spawn(global_executor(), fut)
}

/// Use global register [syscall](rasi_syscall::Executor) to run a future to completion on the current thread.
///
/// This function will block the caller until the given future has completed.
pub fn block_on<Fut, R>(fut: Fut) -> R
where
    Fut: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    syscall_block_on(global_executor(), fut)
}
