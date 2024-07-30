use crate::primitives::{balance::ParseDecimalError, hex::HexError};

/// web3rs error variants.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ParseDecimalError(ParseDecimalError),

    #[error(transparent)]
    HexError(HexError),
}

/// Result type for web3rs.
pub type Result<T> = std::result::Result<T, Error>;
