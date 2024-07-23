use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use mio::Token;

/// The hashed time wheel implementation to handle a massive set of timer tracking tasks.
pub(crate) struct Timewheel {
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
    pub(crate) fn new(tick_interval: Duration) -> Self {
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
    pub(crate) fn new_timer(&self, token: Token, deadline: Instant) -> Option<u64> {
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
    pub(crate) fn next_tick(&self) -> Option<Vec<Token>> {
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
