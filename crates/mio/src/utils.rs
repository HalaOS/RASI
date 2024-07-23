use std::{
    io,
    task::{Poll, Waker},
};

use mio::{Interest, Token};

use crate::reactor::global_reactor;

/// Create a [`CancelablePoll`] instance from [`std::io::Result`] that returns by function `f`.
///
/// If the function `f` result error is [`Interrupted`](io::ErrorKind::Interrupted),
/// `would_block` will call `f` again immediately.
pub(crate) fn would_block<T, F>(
    token: Token,
    waker: Waker,
    interests: Interest,
    mut f: F,
) -> Poll<io::Result<T>>
where
    F: FnMut() -> io::Result<T>,
{
    global_reactor().once(token, interests, waker);

    loop {
        match f() {
            Ok(t) => {
                return {
                    global_reactor().remove_listeners(token, interests);

                    Poll::Ready(Ok(t))
                }
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                return Poll::Pending;
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                continue;
            }
            Err(err) => {
                global_reactor().remove_listeners(token, interests);
                return Poll::Ready(Err(err));
            }
        }
    }
}

pub fn ready<T, F>(f: F) -> Poll<T>
where
    F: FnOnce() -> T,
{
    Poll::Ready(f())
}
