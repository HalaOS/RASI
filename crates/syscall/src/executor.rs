//! syscall for async task.

use std::sync::OnceLock;

use futures::{future::BoxFuture, Future, SinkExt, StreamExt};

/// Future task system call interface.
pub trait Executor: Send + Sync {
    /// Spawns a task that polls the given `boxed` future with output () to completion.
    fn spawn_boxed(&self, fut: BoxFuture<'static, ()>);
}

/// An extension trait which adds utility methods to [`Executor`] types.
pub trait ExecutorExt: Executor {
    /// Spawns a task that polls the given future with output () to completion.
    fn spawn<Fut>(&self, fut: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.spawn_boxed(Box::pin(fut))
    }

    /// Run a future to completion on the current thread.
    ///
    /// This function will block the caller until the given future has completed.
    fn block_on<Fut, R>(&self, fut: Fut) -> R
    where
        Fut: Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        let (mut sender, mut receiver) = futures::channel::mpsc::channel::<R>(0);

        self.spawn(async move {
            let r = fut.await;
            _ = sender.send(r).await;
        });

        futures::executor::block_on(async move { receiver.next().await.unwrap() })
    }
}

impl<T> ExecutorExt for T where T: Executor {}

static GLOBAL_EXECUTOR: OnceLock<Box<dyn Executor>> = OnceLock::new();

/// Register provided [`Executor`] as global executor implementation.
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_global_executor<E: Executor + 'static>(executor: E) {
    if GLOBAL_EXECUTOR.set(Box::new(executor)).is_err() {
        panic!("Multiple calls to register_global_executor are not permitted!!!");
    }
}

/// Get global register [`Executor`] syscall interface.
///
/// # Panic
///
/// You should call [`register_global_executor`] first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_executor first`
pub fn global_executor() -> &'static dyn Executor {
    GLOBAL_EXECUTOR
        .get()
        .expect("Call register_global_executor first")
        .as_ref()
}
