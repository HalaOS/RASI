//! The utility tools to help batch polling futures.

use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
    fmt::Debug,
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

#[cfg(feature = "trace_batching")]
use std::panic::Location;

use futures::{future::BoxFuture, FutureExt, Stream};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct FutureKey {
    id: usize,
    #[cfg(feature = "trace_batching")]
    caller: &'static Location<'static>,
}

impl FutureKey {
    #[cfg(not(feature = "trace_batching"))]
    fn next() -> Self {
        static TOKEN_GEN: AtomicUsize = AtomicUsize::new(0);

        FutureKey {
            id: TOKEN_GEN.fetch_add(1, Ordering::Relaxed),
        }
    }

    #[cfg(feature = "trace_batching")]
    fn next(caller: &'static Location<'static>) -> Self {
        use std::sync::OnceLock;

        use rand::{thread_rng, RngCore};

        static TOKEN_GEN: OnceLock<AtomicUsize> = OnceLock::new();

        let gen = TOKEN_GEN.get_or_init(|| {
            let mut buf = [0u8; 8];
            thread_rng().fill_bytes(&mut buf);

            AtomicUsize::new(u64::from_be_bytes(buf) as usize)
        });

        FutureKey {
            id: gen.fetch_add(1, Ordering::Relaxed),
            caller,
        }
    }
}

impl Debug for FutureKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "trace_batching")]
        {
            write!(
                f,
                "FutureKey, id={}, caller: {}({})",
                self.id,
                self.caller.file(),
                self.caller.line()
            )
        }

        #[cfg(not(feature = "trace_batching"))]
        {
            write!(f, "FutureKey, id={}", self.id)
        }
    }
}

/// The primitive type that handle a group of specific type futures.
///
/// The R is the item type of the never end [`Stream`].
pub struct Group<R> {
    /// The pending futures mapping of this group.
    pending: Arc<parking_lot::Mutex<HashMap<FutureKey, BoxFuture<'static, R>>>>,
    /// A proxy type that handle [`Waker`] instance and ready fifo queue.
    wake_by_key: Arc<parking_lot::Mutex<WakeByKey>>,
    /// Group close flag.
    closed: Arc<AtomicBool>,
}

impl<R> Clone for Group<R> {
    fn clone(&self) -> Self {
        Self {
            pending: self.pending.clone(),
            wake_by_key: self.wake_by_key.clone(),
            closed: self.closed.clone(),
        }
    }
}

impl<R> Group<R> {
    /// Create `FutureGroup` with default config.
    pub fn new() -> (Self, Ready<R>) {
        let this = Self {
            pending: Default::default(),
            wake_by_key: Default::default(),
            closed: Default::default(),
        };

        let ready = Ready {
            pending: this.pending.clone(),
            wake_by_key: this.wake_by_key.clone(),
            closed: this.closed.clone(),
        };

        (this, ready)
    }

    /// Add `fut` to the batch poll group.
    ///
    /// The returns type is [`FutureKey`], you can use it to [`remove`](Self::leave) joined `fut`
    #[track_caller]
    pub fn join<Fut>(&self, fut: Fut) -> FutureKey
    where
        Fut: Future<Output = R> + Send + 'static,
    {
        #[cfg(feature = "trace_batching")]
        let key = FutureKey::next(Location::caller());

        #[cfg(not(feature = "trace_batching"))]
        let key = FutureKey::next();

        self.pending.lock().insert(key, Box::pin(fut));

        self.wake_by_key.lock().wake_by_future_key(key);

        key
    }

    /// Remove a pending future by [`FutureKey`] from this group.
    pub fn leave<Q: Borrow<FutureKey>>(&self, key: Q) -> bool {
        self.pending.lock().remove(key.borrow()).is_some()
    }

    /// Close this group and cancel [`Ready`] stream.
    pub fn close(&self) {
        if self
            .closed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            self.wake_by_key.lock().wake();
        }
    }
}

/// The ready stream of batch poll [`Group`].
pub struct Ready<R> {
    /// The pending futures mapping of this group.
    pending: Arc<parking_lot::Mutex<HashMap<FutureKey, BoxFuture<'static, R>>>>,
    /// A proxy type that handle [`Waker`] instance and ready fifo queue.
    wake_by_key: Arc<parking_lot::Mutex<WakeByKey>>,
    /// Group close flag.
    closed: Arc<AtomicBool>,
}

impl<R> Ready<R> {
    fn batch_poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<R> {
        self.wake_by_key.lock().waker = Some(cx.waker().clone());

        loop {
            let top = self.wake_by_key.lock().ready.pop_front();

            if let Some(key) = top {
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
                        #[cfg(feature = "trace_batching")]
                        log::trace!("Batching: pending {:?}", key);
                        self.pending.lock().insert(key, fut);
                    }
                    Poll::Ready(r) => {
                        #[cfg(feature = "trace_batching")]
                        log::trace!("Batching: ready {:?}", key);
                        return Poll::Ready(r);
                    }
                }
            } else {
                break;
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
        if self.closed.load(Ordering::Acquire) {
            return Poll::Ready(None);
        }

        // self.wake_by_key.pending(cx.waker().clone());

        match self.as_mut().batch_poll(cx) {
            std::task::Poll::Ready(r) => {
                // may no need to call remove_inner_waker
                // drop(self.wake_by_key.remove_inner_waker());

                return Poll::Ready(Some(r));
            }
            std::task::Poll::Pending => {}
        }

        Poll::Pending
    }
}

#[derive(Default)]
struct WakeByKey {
    /// system waker passed by [`Self::pending`] function.
    waker: Option<Waker>,
    /// The [`id`](Token) fifo queue for ready futures.
    ready: VecDeque<FutureKey>,
}

impl WakeByKey {
    fn wake_by_future_key(&mut self, key: FutureKey) {
        self.ready.push_back(key);

        #[cfg(feature = "trace_batching")]
        log::trace!("Batching: wake {:?}", key);

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

#[derive(Clone)]
struct FutureKeyWaker {
    key: FutureKey,
    /// A proxy type that handle [`Waker`] instance and ready fifo queue.
    wake_by_key: Arc<parking_lot::Mutex<WakeByKey>>,
}

impl FutureKeyWaker {
    fn new(key: FutureKey, wake_by_key: Arc<parking_lot::Mutex<WakeByKey>>) -> Self {
        Self { key, wake_by_key }
    }
}

impl cooked_waker::WakeRef for FutureKeyWaker {
    fn wake_by_ref(&self) {
        self.wake_by_key.lock().wake_by_future_key(self.key);
    }
}

impl cooked_waker::Wake for Box<FutureKeyWaker> {}

#[cfg(test)]
mod tests {

    use std::{sync::mpsc, time::Duration};

    use futures::{
        executor::ThreadPool,
        future::{pending, poll_fn},
        task::SpawnExt,
        StreamExt,
    };
    use futures_test::task::noop_context;

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

        assert!(batch_group.wake_by_key.lock().waker.is_none());

        let handle = thread_pool
            .spawn_with_handle(async move {
                for _ in 0..loops {
                    ready.next().await;
                }
            })
            .unwrap();

        // wait
        std::thread::sleep(Duration::from_secs(1));

        assert!(batch_group.wake_by_key.lock().waker.is_some());

        std::thread::spawn(move || {
            for _ in 0..loops {
                receiver.recv().unwrap().wake();
            }
        });

        handle.await;
    }

    #[futures_test::test]
    async fn test_close_group() {
        let (batch_group, mut ready) = Group::<i32>::new();

        batch_group.join(pending());

        assert!(ready.poll_next_unpin(&mut noop_context()).is_pending());

        batch_group.close();

        assert!(ready.next().await.is_none());
    }
}
