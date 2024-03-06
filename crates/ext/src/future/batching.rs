//! The utility tools to help batch polling futures.

use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

use rasi::futures::{future::BoxFuture, FutureExt, Stream};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FutureKey(usize);

impl FutureKey {
    fn next() -> Self {
        static TOKEN_GEN: AtomicUsize = AtomicUsize::new(0);

        FutureKey(TOKEN_GEN.fetch_add(1, Ordering::Relaxed))
    }
}

/// The primitive type that handle a group of specific type futures.
///
/// The R is the item type of the never end [`Stream`].
pub struct Group<R> {
    /// The pending futures mapping of this group.
    pending: Arc<parking_lot::Mutex<HashMap<FutureKey, BoxFuture<'static, R>>>>,
    /// A proxy type that handle [`Waker`] instance and ready fifo queue.
    wake_by_key: Arc<WakeByKey>,
}

impl<R> Clone for Group<R> {
    fn clone(&self) -> Self {
        Self {
            pending: self.pending.clone(),
            wake_by_key: self.wake_by_key.clone(),
        }
    }
}

impl<R> Group<R> {
    /// Create `FutureGroup` with default config.
    pub fn new() -> (Self, Ready<R>) {
        let this = Self {
            pending: Default::default(),
            wake_by_key: Default::default(),
        };

        let ready = Ready {
            pending: this.pending.clone(),
            wake_by_key: this.wake_by_key.clone(),
        };

        (this, ready)
    }

    /// Add `fut` to the batch poll group.
    ///
    /// The returns type is [`FutureKey`], you can use it to [`remove`](Self::leave) joined `fut`
    pub fn join<Fut>(&self, fut: Fut) -> FutureKey
    where
        Fut: Future<Output = R> + Send + 'static,
    {
        let key = FutureKey::next();

        self.pending.lock().insert(key, Box::pin(fut));

        self.wake_by_key.wake(key);

        key
    }

    /// Remove a pending future by [`FutureKey`] from this group.
    pub fn leave<Q: Borrow<FutureKey>>(&self, key: Q) -> bool {
        self.pending.lock().remove(key.borrow()).is_some()
    }
}

/// The ready stream of batch poll [`Group`].
pub struct Ready<R> {
    /// The pending futures mapping of this group.
    pending: Arc<parking_lot::Mutex<HashMap<FutureKey, BoxFuture<'static, R>>>>,
    /// A proxy type that handle [`Waker`] instance and ready fifo queue.
    wake_by_key: Arc<WakeByKey>,
}

impl<R> Ready<R> {
    fn batch_poll(self: std::pin::Pin<&mut Self>) -> std::task::Poll<R> {
        while let Some(key) = self.wake_by_key.pop() {
            let fut = self.pending.lock().remove(&key);

            if fut.is_none() {
                continue;
            }

            let mut fut = fut.unwrap();

            use cooked_waker::IntoWaker;

            let waker: Waker =
                Box::new(FutureKeyWaker::new(key, self.wake_by_key.clone())).into_waker();

            match fut.poll_unpin(&mut Context::from_waker(&waker)) {
                Poll::Pending => {
                    self.pending.lock().insert(key, fut);
                }
                Poll::Ready(r) => {
                    return Poll::Ready(r);
                }
            }
        }

        Poll::Pending
    }
}

impl<R> Stream for Ready<R> {
    type Item = R;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.as_mut().batch_poll() {
            std::task::Poll::Ready(r) => return Poll::Ready(Some(r)),
            std::task::Poll::Pending => {}
        }

        self.wake_by_key.pending(cx.waker().clone());

        match self.as_mut().batch_poll() {
            std::task::Poll::Ready(r) => {
                // may no need to call remove_inner_waker
                drop(self.wake_by_key.remove_inner_waker());

                return Poll::Ready(Some(r));
            }
            std::task::Poll::Pending => {}
        }

        Poll::Pending
    }
}

#[derive(Default)]
struct RawWakeByKey {
    /// system waker passed by [`Self::pending`] function.
    waker: Option<Waker>,
    /// The [`id`](Token) fifo queue for ready futures.
    ready: VecDeque<FutureKey>,
}

#[derive(Default)]
struct WakeByKey {
    raw: parking_lot::Mutex<RawWakeByKey>,
}

impl WakeByKey {
    fn pending(&self, waker: Waker) {
        self.raw.lock().waker = Some(waker);
    }

    fn remove_inner_waker(&self) -> Option<Waker> {
        self.raw.lock().waker.take()
    }

    fn wake(&self, key: FutureKey) {
        let mut raw = self.raw.lock();

        raw.ready.push_back(key);

        if let Some(waker) = raw.waker.take() {
            waker.wake();
        }
    }

    fn pop(&self) -> Option<FutureKey> {
        self.raw.lock().ready.pop_front()
    }
}

#[derive(Clone)]
struct FutureKeyWaker {
    key: FutureKey,
    /// A proxy type that handle [`Waker`] instance and ready fifo queue.
    wake_by_key: Arc<WakeByKey>,
}

impl FutureKeyWaker {
    fn new(key: FutureKey, wake_by_key: Arc<WakeByKey>) -> Self {
        Self { key, wake_by_key }
    }
}

impl cooked_waker::WakeRef for FutureKeyWaker {
    fn wake_by_ref(&self) {
        self.wake_by_key.wake(self.key);
    }
}

impl cooked_waker::Wake for Box<FutureKeyWaker> {}

#[cfg(test)]
mod tests {

    use std::{sync::mpsc, time::Duration};

    use rasi::futures::{
        executor::ThreadPool,
        future::{pending, poll_fn},
        task::SpawnExt,
        StreamExt,
    };

    use super::*;

    #[futures_test::test]
    async fn test_batch_poll() {
        let (batch_group, mut ready) = Group::<i32>::new();

        for _ in 0..100000 {
            batch_group.join(async { 1 });
            batch_group.join(async {
                pending::<()>().await;

                2
            });

            batch_group.join(async { 3 });

            assert_ne!(ready.next().await, Some(2));
            assert_ne!(ready.next().await, Some(2));
        }
    }

    #[futures_test::test]
    async fn test_join_wakeup() {
        let thread_pool = ThreadPool::new().unwrap();

        let (batch_group, mut ready) = Group::<i32>::new();

        for i in 0..100000 {
            let batch_group_cloned = batch_group.clone();

            thread_pool
                .spawn(async move {
                    batch_group_cloned.join(async move { i });
                })
                .unwrap();

            assert_eq!(ready.next().await, Some(i));
        }
    }

    #[futures_test::test]
    async fn test_delay_ready() {
        let (batch_group, mut ready) = Group::<i32>::new();

        let (sender, receiver) = mpsc::channel();

        let loops = 100000;

        for i in 0..loops {
            let mut sent = false;

            let sender = sender.clone();

            batch_group.join(poll_fn(move |cx| {
                if sent {
                    return Poll::Ready(i);
                }

                sender.send(cx.waker().clone()).unwrap();

                sent = true;

                Poll::Pending
            }));
        }

        let thread_pool = ThreadPool::new().unwrap();

        assert!(batch_group.wake_by_key.raw.lock().waker.is_none());

        let handle = thread_pool
            .spawn_with_handle(async move {
                for _ in 0..loops {
                    ready.next().await;
                }
            })
            .unwrap();

        // wait
        std::thread::sleep(Duration::from_secs(1));

        assert!(batch_group.wake_by_key.raw.lock().waker.is_some());

        std::thread::spawn(move || {
            for _ in 0..loops {
                receiver.recv().unwrap().wake();
            }
        });

        handle.await;
    }
}
