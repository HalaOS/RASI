use std::{
    io::Result,
    sync::{Arc, OnceLock},
    task::Waker,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use mio::{event, Interest, Token};

use crate::timewheel::Timewheel;

/// A reactor pattern implementation based on [`mio::Poll`].
///
/// **Note**: This type implement with lockfree structures,
/// so it is only available on platforms that support atomic operations.
pub(crate) struct Reactor {
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
    fn new(tick_interval: Duration) -> Result<ArcReactor> {
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
    pub fn register<S>(&self, source: &mut S, token: Token, interests: Interest) -> Result<()>
    where
        S: event::Source + ?Sized,
    {
        self.mio_registry.register(source, token, interests)
    }

    /// Deregister an [`event::Source`] from the underlying [`mio::Poll`] instance.
    pub fn deregister<S>(&self, source: &mut S) -> Result<()>
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
pub(crate) fn global_reactor() -> ArcReactor {
    GLOBAL_REACTOR
        .get_or_init(|| Reactor::new(Duration::from_millis(10)).unwrap())
        .clone()
}
