use crate::primitives::{balance::ParseBalanceError, HexError};

/// web3rs error variants.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ParseDecimalError(ParseBalanceError),

    #[error(transparent)]
    HexError(HexError),
}

/// Result type for web3rs.
pub type Result<T> = std::result::Result<T, Error>;
