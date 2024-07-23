use std::{
    borrow::Borrow,
    collections::HashMap,
    future::Future,
    hash::Hash,
    task::{Poll, Waker},
};

use futures::{
    lock::{Mutex, MutexLockFuture},
    FutureExt,
};

enum Event<V> {
    Value(V),
    Cancel,
}

struct RawMap<K, V> {
    /// kv store.
    kv: HashMap<K, Event<V>>,
    /// Waiting future wakers.
    wakers: HashMap<K, Waker>,
}

/// A future-based concurrent event map with `wait` API.

pub struct WaitMap<K, V> {
    inner: Mutex<RawMap<K, V>>,
}

impl<K, V> WaitMap<K, V>
where
    K: Eq + Hash + Unpin,
{
    pub fn new() -> Self {
        WaitMap {
            inner: Mutex::new(RawMap {
                kv: HashMap::new(),
                wakers: HashMap::new(),
            }),
        }
    }
    /// Inserts a event-value pair into the map.
    ///
    /// If the map did not have this key present, None is returned.
    pub async fn insert(&self, k: K, v: V) -> Option<V> {
        let mut raw = self.inner.lock().await;

        if let Some(waker) = raw.wakers.remove(&k) {
            waker.wake();
        }

        if let Some(event) = raw.kv.insert(k, Event::Value(v)) {
            match event {
                Event::Value(value) => Some(value),
                Event::Cancel => None,
            }
        } else {
            None
        }
    }

    /// Create a key waiting task until a value is put under the key,
    /// only one waiting task for a key can exist at a time.
    ///
    /// Returns the value at the key if the key was inserted into the map,
    /// or returns `None` if the waiting task is canceled.
    pub async fn wait(&self, k: &K) -> Option<V>
    where
        K: Clone,
    {
        {
            let mut raw = self.inner.lock().await;

            if let Some(v) = raw.kv.remove(&k) {
                match v {
                    Event::Value(value) => return Some(value),
                    Event::Cancel => return None,
                }
            }
        }

        Wait {
            event_map: self,
            lock: None,
            k,
        }
        .await
    }

    /// Cancel other key waiting tasks.
    pub async fn cancel<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut raw = self.inner.lock().await;

        if let Some((k, waker)) = raw.wakers.remove_entry(k) {
            raw.kv.insert(k, Event::Cancel);
            waker.wake();
            true
        } else {
            raw.kv.remove(k);
            false
        }
    }

    /// Cancel all key waiting tasks.
    pub async fn cancel_all(&self) {
        let mut raw = self.inner.lock().await;

        let wakers = raw.wakers.drain().collect::<Vec<_>>();

        for (k, waker) in wakers {
            raw.kv.insert(k, Event::Cancel);
            waker.wake();
        }
    }
}

struct Wait<'a, K, V> {
    event_map: &'a WaitMap<K, V>,
    lock: Option<MutexLockFuture<'a, RawMap<K, V>>>,
    k: &'a K,
}

impl<'a, K, V> Future for Wait<'a, K, V>
where
    K: Eq + Hash + Unpin + Clone,
{
    type Output = Option<V>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut lock = if let Some(lock) = self.lock.take() {
            lock
        } else {
            self.event_map.inner.lock()
        };

        match lock.poll_unpin(cx) {
            std::task::Poll::Ready(mut inner) => {
                if let Some(event) = inner.kv.remove(&self.k) {
                    match event {
                        Event::Value(value) => return Poll::Ready(Some(value)),
                        Event::Cancel => return Poll::Ready(None),
                    }
                } else {
                    inner.wakers.insert(self.k.clone(), cx.waker().clone());
                    return Poll::Pending;
                }
            }
            std::task::Poll::Pending => {
                self.lock = Some(lock);
                return Poll::Pending;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::poll;

    use super::*;

    #[futures_test::test]
    async fn test_event_map() {
        let event_map = WaitMap::<usize, usize>::new();

        event_map.insert(1, 1).await;

        assert_eq!(event_map.wait(&1).await, Some(1));

        let mut wait = Box::pin(event_map.wait(&2));

        assert_eq!(poll!(&mut wait), Poll::Pending);

        event_map.cancel(&2).await;

        assert_eq!(poll!(&mut wait), Poll::Ready(None));

        let mut wait = Box::pin(event_map.wait(&2));

        assert_eq!(poll!(&mut wait), Poll::Pending);

        event_map.insert(2, 2).await;

        assert_eq!(poll!(&mut wait), Poll::Ready(Some(2)));
    }
}
