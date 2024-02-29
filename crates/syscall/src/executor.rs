//! syscall for async task.

use futures::{future::BoxFuture, Future, SinkExt, StreamExt};

/// Future task system call interface.
pub trait Executor {
    /// Spawns a `boxed` task that polls the given future with output () to completion.
    fn spawn_boxed(&self, fut: BoxFuture<'static, ()>);

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
