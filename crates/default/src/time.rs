//! The implementation of [`Timer`] syscall.
//!
//! You can register [`MioTimer`] to the global registry using [`register_mio_timer`], or use it alone.

use std::{
    task::{Poll, Waker},
    time::Instant,
};

use mio::Token;
use rasi_syscall::{register_global_timer, Handle, Timer};

use crate::{reactor::global_reactor, TokenSequence};

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

        Ok(global_reactor()
            .deadline(token, waker, deadline)
            .map(|_| TimerHandle::new(token, deadline)))
    }

    fn timeout_wait(
        &self,
        waker: std::task::Waker,
        handle: &rasi_syscall::Handle,
    ) -> std::task::Poll<()> {
        let time_handle = handle
            .downcast::<TimerHandle>()
            .expect("Expect TimeHandle.");

        match global_reactor().deadline(time_handle.token, waker, time_handle.deadline) {
            Some(_) => Poll::Pending,
            None => Poll::Ready(()),
        }
    }
}

/// This function using [`register_global_timer`] to register the [`MioTimer`] to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_global_timer`)
pub fn register_mio_timer() {
    register_global_timer(MioTimer::default())
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use rasi_spec::timer::run_timer_spec;

    use super::MioTimer;

    static INIT: OnceLock<Box<dyn rasi_syscall::Timer>> = OnceLock::new();

    fn get_syscall() -> &'static dyn rasi_syscall::Timer {
        INIT.get_or_init(|| Box::new(MioTimer::default())).as_ref()
    }

    #[futures_test::test]
    async fn test_timeout() {
        run_timer_spec(get_syscall()).await;
    }
}
