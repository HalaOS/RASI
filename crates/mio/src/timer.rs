//! This mod implement the syscall [`Timer`].
//!
//! You can register [`MioTimer`] to the global registry using [`register_mio_timer`], or use it alone.

use std::{
    task::{Poll, Waker},
    time::Instant,
};

use mio::Token;
use rasi_syscall::{register_global_timer, Handle, Timer};

use crate::{poller::get_global_reactor, TokenSequence};

pub(crate) struct TimerHandle {
    deadline: Instant,
    token: Token,
}

impl TimerHandle {
    pub(crate) fn new(token: Token, deadline: Instant) -> Handle {
        Handle::new(TimerHandle { token, deadline })
    }
}

/// The type that implement the syscall [`Timer`].
#[derive(Debug, Default)]
pub struct MioTimer {}

impl Timer for MioTimer {
    fn deadline(
        &self,
        waker: Waker,
        deadline: std::time::Instant,
    ) -> std::io::Result<Option<rasi_syscall::Handle>> {
        let token = Token::next();

        Ok(get_global_reactor()
            .deadline(token, waker, deadline)
            .map(|_| Handle::new(TimerHandle::new(token, deadline))))
    }

    fn timeout_wait(
        &self,
        waker: std::task::Waker,
        handle: &rasi_syscall::Handle,
    ) -> std::task::Poll<()> {
        let time_handle = handle
            .downcast::<TimerHandle>()
            .expect("Expect TimeHandle.");

        match get_global_reactor().deadline(time_handle.token, waker, time_handle.deadline) {
            Some(_) => Poll::Pending,
            None => Poll::Ready(()),
        }
    }
}

/// This function using [`register_global_timer`] to register the [`MioTimer`] implementation to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_global_timer`)
pub fn register_mio_timer() {
    register_global_timer(MioTimer::default())
}
