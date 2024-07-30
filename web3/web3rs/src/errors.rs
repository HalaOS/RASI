use std::num::ParseIntError;

use crate::primitives::{balance::ParseBalanceError, hex::HexError};

/// web3rs error variants.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ParseDecimalError(#[from] ParseBalanceError),

    #[error(transparent)]
    HexError(#[from] HexError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
}

/// Result type for web3rs.
pub type Result<T> = std::result::Result<T, Error>;
