use std::num::ParseIntError;

use crate::{
    primitives::{balance::ParseBalanceError, HexError},
    rlp::RlpError,
};

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

    #[error(transparent)]
    RlpError(#[from] RlpError),

    #[error("{0}")]
    Other(String),
}

/// Result type for web3rs.
pub type Result<T> = std::result::Result<T, Error>;
