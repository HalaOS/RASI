use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::Debug,
    future::Future,
    hash::Hash,
    sync::Mutex,
    task::{Poll, Waker},
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

pub struct KeyWaitMap<K, V> {
    inner: Mutex<RawMap<K, V>>,
}

impl<K, V> KeyWaitMap<K, V>
where
    K: Eq + Hash + Unpin,
{
    pub fn new() -> Self {
        KeyWaitMap {
            inner: Mutex::new(RawMap {
                kv: HashMap::new(),
                wakers: HashMap::new(),
            }),
        }
    }
    /// Inserts a event-value pair into the map.
    ///
    /// If the map did not have this key present, None is returned.
    pub fn insert(&self, k: K, v: V) -> Option<V> {
        let mut raw = self.inner.lock().unwrap();

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

    /// Inserts a event-value pair into the map.
    ///
    /// If the map did not have this key present, None is returned.
    pub fn batch_insert<I>(&self, kv: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: Debug,
    {
        let mut raw = self.inner.lock().unwrap();

        for (k, v) in kv.into_iter() {
            if let Some(waker) = raw.wakers.remove(&k) {
                log::trace!("wakeup: {:?}", k);
                waker.wake();
            } else {
                log::trace!("wakeup: {:?}, without waiting task", k);
            }

            raw.kv.insert(k, Event::Value(v));
        }
    }

    /// Create a key waiting task until a value is put under the key,
    /// only one waiting task for a key can exist at a time.
    ///
    /// Returns the value at the key if the key was inserted into the map,
    /// or returns `None` if the waiting task is canceled.
    pub async fn wait<L>(&self, k: &K, locker: L) -> Option<V>
    where
        K: Clone,
        L: Unpin,
    {
        Wait {
            event_map: self,
            k,
            locker: Some(locker),
        }
        .await
    }

    /// Cancel other key waiting tasks.
    pub fn cancel<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut raw = self.inner.lock().unwrap();

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
    pub fn cancel_all(&self) {
        let mut raw = self.inner.lock().unwrap();

        let wakers = raw.wakers.drain().collect::<Vec<_>>();

        for (k, waker) in wakers {
            raw.kv.insert(k, Event::Cancel);
            waker.wake();
        }
    }
}

struct Wait<'a, K, V, L> {
    event_map: &'a KeyWaitMap<K, V>,
    k: &'a K,
    locker: Option<L>,
}

impl<'a, K, V, L> Future for Wait<'a, K, V, L>
where
    K: Eq + Hash + Unpin + Clone,
    L: Unpin,
{
    type Output = Option<V>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut inner = self.event_map.inner.lock().unwrap();

        drop(self.locker.take());

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
}

#[cfg(test)]
mod tests {
    use futures::poll;

    use super::*;

    #[futures_test::test]
    async fn test_event_map() {
        let event_map = KeyWaitMap::<usize, usize>::new();

        event_map.insert(1, 1);

        assert_eq!(event_map.wait(&1, ()).await, Some(1));

        let mut wait = Box::pin(event_map.wait(&2, ()));

        assert_eq!(poll!(&mut wait), Poll::Pending);

        event_map.cancel(&2);

        assert_eq!(poll!(&mut wait), Poll::Ready(None));

        let mut wait = Box::pin(event_map.wait(&2, ()));

        assert_eq!(poll!(&mut wait), Poll::Pending);

        event_map.insert(2, 2);

        assert_eq!(poll!(&mut wait), Poll::Ready(Some(2)));
    }
}
