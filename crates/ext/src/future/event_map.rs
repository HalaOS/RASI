//! A mediator pattern implementation for rust async rt.

use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    task::{Poll, Waker},
};

use crate::utils::{Lockable, SpinMutex};

/// The variant for event listener waiting status .
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum EventStatus {
    Pending = 0,
    Ready = 1,
    Cancel = 2,
    Destroy = 3,
}

impl From<EventStatus> for u8 {
    fn from(value: EventStatus) -> Self {
        value as u8
    }
}

impl From<u8> for EventStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => EventStatus::Pending,
            1 => EventStatus::Ready,
            2 => EventStatus::Cancel,
            3 => EventStatus::Destroy,
            _ => panic!("invalid status value: {}", value),
        }
    }
}

/// A mediator pattern implementation for rust async rt.
///
pub struct EventMap<E>
where
    E: Eq + Hash,
{
    listeners: SpinMutex<(bool, HashMap<E, Listener>)>,
}

impl<E> Default for EventMap<E>
where
    E: Eq + Hash + Unpin + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<E> EventMap<E>
where
    E: Eq + Hash + Unpin + Debug + Unpin,
{
    /// Create new [`EventMap<E>`](EventMap) instance with default config.
    pub fn new() -> Self {
        Self {
            listeners: Default::default(),
        }
    }

    /// Listens for the `event` to be triggered once.
    ///
    /// # Parameters
    /// - guard: An RAII guard returned by some lock primitives.
    pub async fn once<G>(&self, event: E, guard: G) -> Result<(), EventStatus>
    where
        G: Unpin,
        E: Clone,
    {
        WaitKey::new(self, event, guard).await
    }

    /// Notify `event` listener, and set the listener status to `status`.
    pub fn notify<Q: Borrow<E>>(&self, event: Q, status: EventStatus) -> bool {
        let mut inner = self.listeners.lock();

        if let Some(listener) = inner.1.get_mut(event.borrow()) {
            listener.status = status;

            listener.waker.wake_by_ref();

            log::trace!("notify {:?}", event.borrow());

            true
        } else {
            log::trace!("notify {:?}, not found", event.borrow());

            false
        }
    }

    /// Notify all provided event listeners on `event_list`, and set the listener status to `status`.
    pub fn notify_all<Q: Borrow<E>, L: AsRef<[Q]>>(&self, event_list: L, status: EventStatus) {
        let mut inner = self.listeners.lock();

        for event in event_list.as_ref() {
            if let Some(listener) = inner.1.get_mut(event.borrow()) {
                listener.status = status;

                listener.waker.wake_by_ref();

                log::trace!("notify {:?}", event.borrow());
            } else {
                log::trace!("notify {:?}, not found", event.borrow());
            }
        }
    }

    pub fn close(&self) {
        let mut inner = self.listeners.lock();

        if inner.0 {
            return;
        }

        inner.0 = true;

        for (_, listener) in inner.1.iter_mut() {
            listener.status = EventStatus::Destroy;

            listener.waker.wake_by_ref();
        }
    }
}

struct Listener {
    waker: Waker,
    status: EventStatus,
}

/// The type of future that is waiting for a specific event to be notified.
#[must_use = "if unused, the event listener will never actually register."]
pub struct WaitKey<'a, E, G>
where
    E: Eq + Hash + Unpin,
{
    event: E,
    event_map: &'a EventMap<E>,
    guard: Option<G>,
}

impl<'a, E, G> WaitKey<'a, E, G>
where
    E: Eq + Hash + Unpin + Debug,
{
    fn new(event_map: &'a EventMap<E>, event: E, guard: G) -> Self {
        Self {
            guard: Some(guard),
            event,
            event_map,
        }
    }
}

impl<'a, E, G> futures::Future for WaitKey<'a, E, G>
where
    E: Eq + Hash + Unpin + Clone + Debug,
    G: Unpin,
{
    type Output = Result<(), EventStatus>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut raw = self.event_map.listeners.lock();

        let status = if let Some(listener) = raw.1.remove(&self.event) {
            listener.status
        } else {
            EventStatus::Pending
        };

        self.guard.take();

        match status {
            EventStatus::Pending => {
                raw.1.insert(
                    self.event.clone(),
                    Listener {
                        waker: cx.waker().clone(),
                        status,
                    },
                );
                // This future may wrapped by select!, so runtime may call this future's poll without really call `Waker::wake()`.
                Poll::Pending
            }
            EventStatus::Ready => Poll::Ready(Ok(())),
            _ => Poll::Ready(Err(status)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread::sleep, time::Duration};

    use futures::{lock, task::SpawnExt};

    use super::*;

    #[futures_test::test]
    async fn test_with_future_aware_mutex() {
        let event_map = Arc::new(EventMap::<i32>::new());

        let locker = Arc::new(futures::lock::Mutex::new(()));

        let guard = locker.lock().await;

        let thread_pool = futures::executor::ThreadPool::new().unwrap();

        let event_map_cloned = event_map.clone();

        let locker_cloned = locker.clone();

        thread_pool
            .spawn(async move {
                locker_cloned.lock().await;
                event_map_cloned.notify(1, EventStatus::Ready);
            })
            .unwrap();

        event_map.once(1, guard).await.unwrap();

        locker.lock().await;
    }

    #[futures_test::test]
    async fn test_with_std_mutex() {
        let event_map = Arc::new(EventMap::<i32>::new());

        let locker = Arc::new(std::sync::Mutex::new(()));

        let guard = locker.lock().unwrap();

        let thread_pool = futures::executor::ThreadPool::new().unwrap();

        let event_map_cloned = event_map.clone();

        let locker_cloned = locker.clone();

        thread_pool
            .spawn(async move {
                let _guard = locker_cloned.lock().unwrap();
                event_map_cloned.notify(1, EventStatus::Ready);
            })
            .unwrap();

        event_map.once(1, guard).await.unwrap();

        let _guard = locker.lock().unwrap();
    }

    #[futures_test::test]
    async fn test_notify_all() {
        let event_map = Arc::new(EventMap::<i32>::new());

        let thread_pool = futures::executor::ThreadPool::new().unwrap();

        let mut handles = vec![];

        let loops = 100;

        for i in 0..loops {
            let event_map = event_map.clone();

            handles.push(
                thread_pool
                    .spawn_with_handle(async move {
                        let locker = lock::Mutex::new(());

                        let guard = locker.lock();

                        event_map.once(i, guard).await.unwrap();
                    })
                    .unwrap(),
            );
        }

        // Waiting for `loop` function `event_map.once` calls to finish
        loop {
            sleep(Duration::from_millis(100));

            if event_map.listeners.lock().1.len() == loops as usize {
                break;
            }
        }

        event_map.notify_all((0..loops).collect::<Vec<_>>(), EventStatus::Ready);

        for (_, handle) in handles.iter_mut().enumerate() {
            handle.await;
        }
    }
}
