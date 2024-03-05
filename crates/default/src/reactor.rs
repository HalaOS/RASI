//! A reactor pattern implementation based on [`mio::Poll`].
//!
//! This is the implementation detail of the `syscall`s and is best not used directly.
//!
//! You can get the global instance of [`Reactor`] by calling function [`get_global_reactor`].
//!
//! # Examples
//!
//! ```no_run
//! # fn main() {
//! #
//! use rasi_default::reactor;
//!
//! let reactor = reactor::get_global_reactor();
//! #
//! # }
//! ```
//!
use std::{
    io,
    ops::Deref,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, OnceLock,
    },
    task::Waker,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use mio::{
    event::{self, Source},
    Interest, Token,
};
use rasi_syscall::{CancelablePoll, Handle};

/// A wrapper of mio event source.
pub(crate) struct MioSocket<S: Source> {
    /// Associcated token.
    pub(crate) token: Token,
    /// net source type.
    pub(crate) socket: S,
}

impl<S: Source> From<(Token, S)> for MioSocket<S> {
    fn from(value: (Token, S)) -> Self {
        Self {
            token: value.0,
            socket: value.1,
        }
    }
}

impl<S: Source> Deref for MioSocket<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.socket
    }
}

impl<S: Source> Drop for MioSocket<S> {
    fn drop(&mut self) {
        if get_global_reactor().deregister(&mut self.socket).is_err() {
            println!("");
        }
    }
}

/// Create a [`CancelablePoll`] instance from [`std::io::Result`] that returns by function `f`.
///
/// If the function `f` result error is [`Interrupted`](io::ErrorKind::Interrupted),
/// `would_block` will call `f` again immediately.
pub(crate) fn would_block<T, F>(
    token: Token,
    waker: Waker,
    interests: Interest,
    mut f: F,
) -> CancelablePoll<io::Result<T>>
where
    F: FnMut() -> io::Result<T>,
{
    get_global_reactor().once(token, interests, waker);

    loop {
        match f() {
            Ok(t) => {
                return {
                    get_global_reactor().remove_listeners(token, interests);

                    CancelablePoll::Ready(Ok(t))
                }
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                return CancelablePoll::Pending(Handle::new(()));
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                continue;
            }
            Err(err) => {
                get_global_reactor().remove_listeners(token, interests);
                return CancelablePoll::Ready(Err(err));
            }
        }
    }
}

/// The hashed time wheel implementation to handle a massive set of timer tracking tasks.
struct Timewheel {
    tick_interval: u64,
    /// The timer map from ticks to timers queue.
    timers: DashMap<u64, boxcar::Vec<Token>>,
    /// Clock ticks that have elapsed since this object created.
    ticks: AtomicU64,
    /// The timestamp of this timewheel instance was created
    start_instant: Instant,
    /// Aliving timers's count.
    timer_count: AtomicU64,
}

impl Timewheel {
    fn new(tick_interval: Duration) -> Self {
        Self {
            tick_interval: tick_interval.as_micros() as u64,
            ticks: Default::default(),
            start_instant: Instant::now(),
            timer_count: Default::default(),
            timers: Default::default(),
        }
    }

    /// Returns aliving timers's count
    #[allow(unused)]
    fn timers(&self) -> u64 {
        self.timer_count.load(Ordering::Relaxed)
    }

    /// Creates a new timer and returns the timer expiration ticks.
    pub fn new_timer(&self, token: Token, deadline: Instant) -> Option<u64> {
        let ticks = (deadline - self.start_instant).as_micros() as u64 / self.tick_interval;

        let ticks = ticks as u64;

        if self
            .ticks
            .fetch_update(Ordering::Release, Ordering::Acquire, |current| {
                if current > ticks {
                    None
                } else {
                    Some(current)
                }
            })
            .is_err()
        {
            return None;
        }

        self.timers.entry(ticks).or_default().push(token);

        if self.ticks.load(Ordering::SeqCst) > ticks {
            if self.timers.remove(&ticks).is_some() {
                return None;
            }
        }

        if self
            .ticks
            .fetch_update(Ordering::Release, Ordering::Acquire, |current| {
                if current > ticks {
                    if self.timers.remove(&ticks).is_some() {
                        return None;
                    } else {
                        return Some(current);
                    }
                } else {
                    return Some(current);
                }
            })
            .is_err()
        {
            return None;
        }

        self.timer_count.fetch_add(1, Ordering::SeqCst);

        Some(ticks)
    }

    /// Forward to next tick, and returns timeout timers.
    pub fn next_tick(&self) -> Option<Vec<Token>> {
        loop {
            let current = self.ticks.load(Ordering::Acquire);

            let instant_duration = Instant::now() - self.start_instant;

            let ticks = instant_duration.as_micros() as u64 / self.tick_interval;

            assert!(current <= ticks);

            if current == ticks {
                return None;
            }

            if self
                .ticks
                .compare_exchange(current, ticks, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                let mut timeout_timers = vec![];

                for i in current..ticks {
                    if let Some((_, queue)) = self.timers.remove(&i) {
                        for t in queue.into_iter() {
                            timeout_timers.push(t);
                        }
                    }
                }

                return Some(timeout_timers);
            }
        }
    }
}

/// A reactor pattern implementation based on [`mio::Poll`].
///
/// **Note**: This type implement with lockfree structures,
/// so it is only available on platforms that support atomic operations.
pub struct Reactor {
    /// Io resources registry.
    mio_registry: mio::Registry,
    /// The pending registry for reading operations.
    read_op_wakers: DashMap<Token, Waker>,
    /// The pending registry for writing operations.
    write_op_wakers: DashMap<Token, Waker>,
    /// hashed time wheel implementation.
    timewheel: Timewheel,
}

/// A thread-safe reference-counting pointer of type [`Reactor`]
pub type ArcReactor = Arc<Reactor>;

impl Reactor {
    fn new(tick_interval: Duration) -> io::Result<ArcReactor> {
        let mio_poll = mio::Poll::new()?;

        let mio_registry = mio_poll.registry().try_clone()?;

        let reactor = Arc::new(Reactor {
            mio_registry,
            read_op_wakers: Default::default(),
            write_op_wakers: Default::default(),
            timewheel: Timewheel::new(tick_interval),
        });

        let background = ReactorBackground::new(tick_interval, mio_poll, reactor.clone());

        background.start();

        Ok(reactor)
    }

    /// Register an [`event::Source`] with the underlying [`mio::Poll`] instance.
    pub fn register<S>(&self, source: &mut S, token: Token, interests: Interest) -> io::Result<()>
    where
        S: event::Source + ?Sized,
    {
        self.mio_registry.register(source, token, interests)
    }

    /// Deregister an [`event::Source`] from the underlying [`mio::Poll`] instance.
    pub fn deregister<S>(&self, source: &mut S) -> io::Result<()>
    where
        S: event::Source + ?Sized,
    {
        self.mio_registry.deregister(source)
    }

    /// Create new `deadline` timer, returns [`None`] if the `deadline` instant is reached.
    pub fn deadline(&self, token: Token, waker: Waker, deadline: Instant) -> Option<u64> {
        self.write_op_wakers.insert(token, waker);

        // Adding a timer was successful.
        if let Some(id) = self.timewheel.new_timer(token, deadline) {
            Some(id)
        } else {
            // Adding a timer fails because the `deadline` has expired.
            //
            // So remove the waker from memory immediately.
            self.write_op_wakers.remove(&token);
            None
        }
    }

    /// Add a [`interests`](Interest) [`listener`](Waker) to this reactor.
    pub fn once(&self, token: Token, interests: Interest, waker: Waker) {
        if interests.is_readable() {
            self.read_op_wakers.insert(token, waker.clone());
        }

        if interests.is_writable() {
            self.write_op_wakers.insert(token, waker);
        }
    }

    /// notify [`listener`](Waker) by [`Token`] and [`interests`](Interest)
    pub fn notify(&self, token: Token, interests: Interest) {
        if interests.is_readable() {
            if let Some(waker) = self.read_op_wakers.remove(&token).map(|(_, v)| v) {
                waker.wake();
            }
        }

        if interests.is_writable() {
            if let Some(waker) = self.write_op_wakers.remove(&token).map(|(_, v)| v) {
                waker.wake();
            }
        }
    }

    /// remove [`listener`](Waker) from this reactor by [`Token`] and [`interests`](Interest)
    pub fn remove_listeners(&self, token: Token, interests: Interest) {
        if interests.is_readable() {
            self.read_op_wakers.remove(&token);
        }

        if interests.is_writable() {
            self.write_op_wakers.remove(&token);
        }
    }
}

/// The context of [`Reactor`] background thread.
struct ReactorBackground {
    mio_poll: mio::Poll,
    reactor: ArcReactor,
    tick_interval: Duration,
}

impl ReactorBackground {
    fn new(tick_interval: Duration, mio_poll: mio::Poll, reactor: ArcReactor) -> Self {
        Self {
            mio_poll,
            reactor,
            tick_interval,
        }
    }

    /// Start readiness events dispatch, and consume self.
    fn start(mut self) {
        std::thread::spawn(move || {
            self.dispatch_loop();
        });
    }

    /// Readiness event dispatch loop.
    fn dispatch_loop(&mut self) {
        let mut events = mio::event::Events::with_capacity(1024);

        loop {
            self.mio_poll
                .poll(&mut events, Some(self.tick_interval))
                .expect("Mio poll panic");

            for event in &events {
                if event.is_readable() {
                    self.notify(event.token(), Interest::READABLE);
                }

                if event.is_writable() {
                    self.notify(event.token(), Interest::WRITABLE);
                }
            }

            let timeout_timers = self.reactor.timewheel.next_tick();

            if let Some(timeout_timers) = timeout_timers {
                for token in timeout_timers {
                    self.notify(token, Interest::WRITABLE);
                }
            }
        }
    }

    fn notify(&self, token: Token, interests: Interest) {
        self.reactor.notify(token, interests);
    }
}

static GLOBAL_REACTOR: OnceLock<ArcReactor> = OnceLock::new();

/// Manually start [`Reactor`] service with providing `tick_interval`.
///
/// If `start_reactor_with` is not called at the very beginning of the `main fn`,
/// [`Reactor`] will run with the default tick_interval = 10ms.
///
/// # Panic
///
/// Call this function more than once or Call this function after calling any
/// [`Network`](rasi_syscall::Network) [`Timer`](rasi_syscall::Timer) system interface , will cause a panic with message
/// `Call start_reactor_with twice.`
pub fn start_reactor_with(tick_interval: Duration) {
    if GLOBAL_REACTOR
        .set(Reactor::new(tick_interval).unwrap())
        .is_err()
    {
        panic!("Call start_reactor_with twice.");
    }
}

/// Get the globally registered instance of [`Reactor`].
///
/// If call this function before calling [`start_reactor_with`],
/// the implementation will start [`Reactor`] with tick_interval = 10ms.
pub fn get_global_reactor() -> ArcReactor {
    GLOBAL_REACTOR
        .get_or_init(|| Reactor::new(Duration::from_millis(10)).unwrap())
        .clone()
}

#[cfg(test)]
mod tests {
    use std::{sync::Barrier, thread::sleep, time::Duration};

    use crate::TokenSequence;

    use super::*;

    #[test]
    fn test_add_timers() {
        let threads = 10;
        let loops = 3usize;

        let time_wheel = Arc::new(Timewheel::new(Duration::from_millis(100)));

        let barrier = Arc::new(Barrier::new(threads));

        let mut handles = vec![];

        for _ in 0..threads {
            let barrier = barrier.clone();

            let time_wheel = time_wheel.clone();

            handles.push(std::thread::spawn(move || {
                barrier.wait();

                for i in 0..loops {
                    time_wheel
                        .new_timer(
                            Token::next(),
                            Instant::now() + Duration::from_secs((i + 1) as u64),
                        )
                        .unwrap();
                }
            }))
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(time_wheel.timers() as usize, threads * loops);

        let mut handles = vec![];

        let counter = Arc::new(AtomicU64::new(0));

        for _ in 0..threads {
            let time_wheel = time_wheel.clone();

            let counter = counter.clone();

            handles.push(std::thread::spawn(move || loop {
                if let Some(timers) = time_wheel.next_tick() {
                    counter.fetch_add(timers.len() as u64, Ordering::SeqCst);
                }

                if counter.load(Ordering::SeqCst) == (threads * loops) as u64 {
                    break;
                }
            }))
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_next_tick() {
        let time_wheel = Timewheel::new(Duration::from_millis(100));

        let token = Token::next();
        assert_eq!(
            time_wheel.new_timer(token, Instant::now() + Duration::from_millis(100)),
            Some(1)
        );

        sleep(Duration::from_millis(200));

        assert_eq!(time_wheel.next_tick(), Some(vec![token]));
    }
}
