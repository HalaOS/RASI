use std::{
    collections::HashMap,
    ops::{self, DerefMut},
    task::Waker,
};

#[cfg(feature = "trace_lock")]
use std::{fmt::Debug, panic::Location};

use super::*;

/// Type factory for [`AsyncLockable`]
pub struct AsyncLockableMaker<Locker, Wakers> {
    inner_locker: Locker,
    wakers: Wakers,
}

impl<Locker, Wakers> Default for AsyncLockableMaker<Locker, Wakers>
where
    Locker: Default,
    Wakers: Default,
{
    fn default() -> Self {
        Self {
            inner_locker: Default::default(),
            wakers: Default::default(),
        }
    }
}

impl<Locker, Wakers> AsyncLockableMaker<Locker, Wakers>
where
    Locker: LockableNew,
    Wakers: Default,
{
    pub fn new(value: Locker::Value) -> Self {
        Self {
            inner_locker: Locker::new(value),
            wakers: Default::default(),
        }
    }
}

impl<Locker, Wakers, Mediator> AsyncLockable for AsyncLockableMaker<Locker, Wakers>
where
    Locker: Lockable + Send + Sync,
    for<'a> Locker::GuardMut<'a>: Send + Unpin,
    Wakers: Lockable + Send + Sync,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator + 'static,
{
    type GuardMut<'a>= AsyncLockableMakerGuard<'a, Locker, Wakers,Mediator>
    where
        Self: 'a;

    type GuardMutFuture<'a> = AsyncLockableMakerFuture<'a, Locker, Wakers,Mediator>
    where
        Self: 'a;

    #[track_caller]
    fn lock(&self) -> Self::GuardMutFuture<'_> {
        AsyncLockableMakerFuture {
            locker: self,
            wait_key: None,
            #[cfg(feature = "trace_lock")]
            caller: Location::caller(),
        }
    }

    fn unlock<'a>(guard: Self::GuardMut<'a>) -> &'a Self {
        let locker = guard.locker;

        drop(guard);

        locker
    }
}

/// RAII `Guard` type for [`AsyncLockableMaker`]
pub struct AsyncLockableMakerGuard<'a, Locker, Wakers, Mediator>
where
    Locker: Lockable,
    Wakers: Lockable,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator,
{
    locker: &'a AsyncLockableMaker<Locker, Wakers>,
    inner_guard: Option<Locker::GuardMut<'a>>,
    #[cfg(feature = "trace_lock")]
    caller: &'static Location<'static>,
}

impl<'a, Locker, Wakers, Mediator> AsyncGuardMut<'a>
    for AsyncLockableMakerGuard<'a, Locker, Wakers, Mediator>
where
    Locker: Lockable + Send + Sync,
    for<'b> Locker::GuardMut<'b>: Send + Unpin,
    Wakers: Lockable + Send + Sync,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator + 'static,
{
    type Locker = AsyncLockableMaker<Locker, Wakers>;
}

impl<'a, Locker, Wakers, Mediator, T> ops::Deref
    for AsyncLockableMakerGuard<'a, Locker, Wakers, Mediator>
where
    Locker: Lockable,
    for<'c> Locker::GuardMut<'c>: ops::Deref<Target = T>,
    Wakers: Lockable,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner_guard.as_deref().unwrap()
    }
}

impl<'a, Locker, Wakers, Mediator, T> ops::DerefMut
    for AsyncLockableMakerGuard<'a, Locker, Wakers, Mediator>
where
    Locker: Lockable,
    for<'c> Locker::GuardMut<'c>: ops::DerefMut<Target = T>,
    Wakers: Lockable,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_guard.as_deref_mut().unwrap()
    }
}

impl<'a, Locker, Wakers, Mediator> Drop for AsyncLockableMakerGuard<'a, Locker, Wakers, Mediator>
where
    Locker: Lockable,
    Wakers: Lockable,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator,
{
    fn drop(&mut self) {
        if let Some(guard) = self.inner_guard.take() {
            drop(guard);

            #[cfg(feature = "trace_lock")]
            log::trace!(
                "async unlocked, caller: {}({})",
                self.caller.file(),
                self.caller.line(),
            );

            let mut wakers = self.locker.wakers.lock();

            wakers.notify_one();
        }
    }
}

/// Future created by [`lock`](AsyncLockableMaker::lock) function.
pub struct AsyncLockableMakerFuture<'a, Locker, Wakers, Mediator>
where
    Locker: Lockable,
    Wakers: Lockable,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator,
{
    locker: &'a AsyncLockableMaker<Locker, Wakers>,
    wait_key: Option<usize>,
    #[cfg(feature = "trace_lock")]
    caller: &'static Location<'static>,
}

#[cfg(feature = "trace_lock")]
impl<'a, Locker, Wakers, Mediator> Debug for AsyncLockableMakerFuture<'a, Locker, Wakers, Mediator>
where
    Locker: Lockable,
    Wakers: Lockable,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "caller: {}({})", self.caller.file(), self.caller.line())
    }
}

impl<'a, Locker, Wakers, Mediator> std::future::Future
    for AsyncLockableMakerFuture<'a, Locker, Wakers, Mediator>
where
    Locker: Lockable,
    Wakers: Lockable,
    for<'b> Wakers::GuardMut<'b>: DerefMut<Target = Mediator>,
    Mediator: AsyncLockableMediator,
{
    type Output = AsyncLockableMakerGuard<'a, Locker, Wakers, Mediator>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        #[cfg(feature = "trace_lock")]
        log::trace!("async try lock, {:?}", self);

        if let Some(guard) = self.locker.inner_locker.try_lock() {
            #[cfg(feature = "trace_lock")]
            log::trace!("async locked, {:?}", self);

            return std::task::Poll::Ready(AsyncLockableMakerGuard {
                locker: self.locker,
                inner_guard: Some(guard),
                #[cfg(feature = "trace_lock")]
                caller: self.caller,
            });
        }

        let mut wakers = self.locker.wakers.lock();

        #[cfg(feature = "trace_lock")]
        {
            self.wait_key = Some(wakers.wait_lockable(cx, self.caller));
        }

        #[cfg(not(feature = "trace_lock"))]
        {
            self.wait_key = Some(wakers.wait_lockable(cx));
        }

        // Ensure that we haven't raced `MutexGuard::drop`'s unlock path by
        // attempting to acquire the lock again
        if let Some(guard) = self.locker.inner_locker.try_lock() {
            #[cfg(feature = "trace_lock")]
            log::trace!("async locked, {:?}", self);

            wakers.cancel(self.wait_key.take().unwrap());

            return std::task::Poll::Ready(AsyncLockableMakerGuard {
                locker: self.locker,
                inner_guard: Some(guard),
                #[cfg(feature = "trace_lock")]
                caller: self.caller,
            });
        }

        std::task::Poll::Pending
    }
}

pub struct DefaultAsyncLockableMediator {
    key_next: usize,
    #[cfg(feature = "trace_lock")]
    wakers: HashMap<usize, (&'static Location<'static>, Waker)>,
    #[cfg(not(feature = "trace_lock"))]
    wakers: HashMap<usize, Waker>,
}

impl Default for DefaultAsyncLockableMediator {
    fn default() -> Self {
        Self {
            key_next: 0,
            wakers: HashMap::new(),
        }
    }
}

impl AsyncLockableMediator for DefaultAsyncLockableMediator {
    #[cfg(not(feature = "trace_lock"))]
    fn wait_lockable(&mut self, cx: &mut std::task::Context<'_>) -> usize {
        let key = self.key_next;
        self.key_next += 1;

        self.wakers.insert(key, cx.waker().clone());

        key
    }

    #[cfg(feature = "trace_lock")]
    fn wait_lockable(
        &mut self,
        cx: &mut std::task::Context<'_>,
        tracer: &'static Location<'static>,
    ) -> usize {
        let key = self.key_next;
        self.key_next += 1;

        log::trace!(
            "async locked pending,caller: {}({}), ptr={:?}",
            tracer.file(),
            tracer.line(),
            self as *mut Self,
        );

        self.wakers.insert(key, (tracer, cx.waker().clone()));

        key
    }

    fn cancel(&mut self, key: usize) -> bool {
        #[cfg(feature = "trace_lock")]
        {
            if let Some((tracer, _)) = self.wakers.remove(&key) {
                log::trace!(
                    "async locked remove pending,caller: {}({}), ptr={:?}",
                    tracer.file(),
                    tracer.line(),
                    self as *mut Self,
                );

                true
            } else {
                false
            }
        }

        #[cfg(not(feature = "trace_lock"))]
        {
            return self.wakers.remove(&key).is_some();
        }
    }

    fn notify_one(&mut self) {
        #[cfg(feature = "trace_lock")]
        log::trace!(
            "waker list notify one. total len={}, ptr={:?}",
            self.wakers.len(),
            self as *mut Self,
        );

        let mut keys = self.wakers.keys().cloned().collect::<Vec<_>>();

        if !keys.is_empty() {
            keys.sort();

            #[cfg(feature = "trace_lock")]
            {
                let (tracer, waker) = self.wakers.remove(&keys[0]).unwrap();
                log::trace!(
                    "notify async lock pending, caller: {}({}), ptr={:?}",
                    tracer.file(),
                    tracer.line(),
                    self as *mut Self,
                );

                waker.wake();
            }

            #[cfg(not(feature = "trace_lock"))]
            {
                let waker = self.wakers.remove(&keys[0]).unwrap();

                waker.wake();
            }
        }
    }

    fn notify_all(&mut self) {
        #[cfg(feature = "trace_lock")]
        for (_, (tracer, waker)) in self.wakers.drain() {
            log::trace!(
                "notify async lock pending, caller: {}({})",
                tracer.file(),
                tracer.line()
            );

            waker.wake();
        }

        #[cfg(not(feature = "trace_lock"))]
        for (_, waker) in self.wakers.drain() {
            waker.wake();
        }
    }
}
