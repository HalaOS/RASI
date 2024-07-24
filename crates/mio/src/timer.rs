//! The implementation of [`Timer`] syscall.
//!
//! You can register [`MioTimer`] to the global registry using [`register_mio_timer`], or use it alone.

use std::{task::Poll, time::Instant};

use mio::Token;
use rasi::timer::register_timer_driver;

use crate::reactor::global_reactor;
use crate::token::TokenSequence;

struct MioDeadLine {
    deadline: Instant,
    token: Token,
}

impl MioDeadLine {
    pub(crate) fn new(token: Token, deadline: Instant) -> rasi::timer::DeadLine {
        MioDeadLine { token, deadline }.into()
    }
}

impl rasi::timer::syscall::DriverDeadLine for MioDeadLine {
    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        match global_reactor().deadline(self.token, cx.waker().clone(), self.deadline) {
            Some(_) => Poll::Pending,
            None => Poll::Ready(()),
        }
    }
}

struct MioTimerDriver;

impl rasi::timer::syscall::Driver for MioTimerDriver {
    fn deadline(&self, deadline: Instant) -> std::io::Result<Option<rasi::timer::DeadLine>> {
        Ok(Some(MioDeadLine::new(Token::next(), deadline)))
    }
}

/// This function using [`register_global_timer`] to register the [`MioTimer`] to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_global_timer`)
pub fn register_mio_timer() {
    register_timer_driver(MioTimerDriver)
}

#[cfg(test)]
mod tests {

    use rasi_spec::timer::run_timer_spec;

    use super::*;

    #[futures_test::test]
    async fn test_timeout() {
        static DRIVER: MioTimerDriver = MioTimerDriver;

        run_timer_spec(&DRIVER).await;
    }
}
