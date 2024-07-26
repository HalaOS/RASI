use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    hash::Hash,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use cooked_waker::{IntoWaker, WakeRef};
use futures::{future::BoxFuture, FutureExt, Stream};

struct RawFutureWaitMap<K, R> {
    futs: HashMap<K, BoxFuture<'static, R>>,
    ready_queue: VecDeque<K>,
    waker: Option<Waker>,
}

impl<K, R> Default for RawFutureWaitMap<K, R> {
    fn default() -> Self {
        Self {
            futs: HashMap::new(),
            ready_queue: VecDeque::new(),
            waker: None,
        }
    }
}

/// A waitable map for futures.
pub struct FutureWaitMap<K, R> {
    inner: Arc<Mutex<RawFutureWaitMap<K, R>>>,
}

impl<K, R> Clone for FutureWaitMap<K, R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K, R> FutureWaitMap<K, R> {
    /// Create a new future `WaitMap` instance.
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }
    /// Insert a new key / future pair.
    pub fn insert<Fut>(&self, k: K, fut: Fut)
    where
        Fut: Future<Output = R> + Send + 'static,
        K: Hash + Eq + Clone,
    {
        let mut inner = self.inner.lock().unwrap();

        inner.ready_queue.push_back(k.clone());
        inner.futs.insert(k, Box::pin(fut));
    }

    pub fn poll_next(&self, cx: &mut Context<'_>) -> Poll<(K, R)>
    where
        K: Hash + Eq + Clone + Send + Sync + 'static,
        R: 'static,
    {
        let cloned = self.inner.clone();

        let mut inner = self.inner.lock().unwrap();

        if let Some(key) = inner.ready_queue.pop_front() {
            let mut fut = inner.futs.remove(&key).expect(
                "Inner error: when insert wake information, checking the futs map is necessarily",
            );

            let waker = Arc::new(FutureWaitMapWaker(key.clone(), cloned)).into_waker();

            let mut proxy_context = Context::from_waker(&waker);

            match fut.poll_unpin(&mut proxy_context) {
                Poll::Ready(r) => return Poll::Ready((key, r)),
                Poll::Pending => {
                    inner.waker = Some(cx.waker().clone());
                    inner.futs.insert(key, fut);
                    return Poll::Pending;
                }
            }
        }

        Poll::Pending
    }
}

impl<K, R> Stream for FutureWaitMap<K, R>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    R: 'static,
{
    type Item = (K, R);

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        FutureWaitMap::poll_next(&self, cx).map(Some)
    }
}

impl<K, R> Stream for &FutureWaitMap<K, R>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    R: 'static,
{
    type Item = (K, R);

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        FutureWaitMap::poll_next(&self, cx).map(Some)
    }
}

struct FutureWaitMapWaker<K, R>(K, Arc<Mutex<RawFutureWaitMap<K, R>>>);

impl<K, R> WakeRef for FutureWaitMapWaker<K, R>
where
    K: Hash + Eq + Clone,
{
    fn wake_by_ref(&self) {
        let mut inner = self.1.lock().unwrap();

        if inner.futs.contains_key(&self.0) {
            inner.ready_queue.push_back(self.0.clone());
            if let Some(waker) = inner.waker.take() {
                waker.wake();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::Poll;

    use futures::{
        future::{pending, poll_fn},
        poll, StreamExt,
    };

    use super::FutureWaitMap;

    #[futures_test::test]
    async fn test_map() {
        let map = FutureWaitMap::new();

        map.insert(1, pending::<i32>());

        let mut map_ref = &map;

        let mut next = map_ref.next();

        assert_eq!(poll!(&mut next), Poll::Pending);

        map.insert(1, poll_fn(|_| Poll::Ready(2)));

        assert_eq!(poll!(&mut next), Poll::Ready(Some((1, 2))));
    }
}
