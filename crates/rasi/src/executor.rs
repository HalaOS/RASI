use futures::Future;
use rasi_syscall::{global_executor, syscall_spawn};

/// Use global register [syscall](rasi_syscall::Executor) to spawn a task
/// that polls the given future with output () to completion.
#[inline]
pub fn spawn<Fut>(fut: Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    syscall_spawn(global_executor(), fut)
}
