use mio::Token;
use std::sync::atomic::{AtomicUsize, Ordering};

/// This trait defines a method for sequential generation of [`token`](Token)s
///
/// **Note**: This type is only available on platforms that support atomic loads and stores of usize
pub trait TokenSequence {
    /// generate next file description handle.
    fn next() -> Token {
        static NEXT: AtomicUsize = AtomicUsize::new(0);

        Token(NEXT.fetch_add(1, Ordering::SeqCst))
    }
}

impl TokenSequence for Token {}
