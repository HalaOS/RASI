use std::num::ParseIntError;

use uint::{FromDecStrErr, FromHexError, FromStrRadixErr};

/// web3rs error variants.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid ethereum unit type: {0}")]
    ParseUnit(String),

    #[error("Invalid deciaml string: {0}")]
    ParseDecimal(String),

    #[error(transparent)]
    FromHexError(#[from] FromHexError),

    #[error(transparent)]
    FromDecStrErr(#[from] FromDecStrErr),

    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error(transparent)]
    ParseDecimalError(ParseDecimalError),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseDecimalError {
    #[error("Decimal base part is empty.")]
    BasePartIsEmpty,

    #[error("Exponent overflow when parsing {0}")]
    ExponentOverflow(String),

    #[error(transparent)]
    FromStrRadixErr(#[from] FromStrRadixErr),

    #[error("Decimal unit part is empty.")]
    Unit,

    #[error("Decimal overflow when parsing {0}")]
    OverFlow(String),
}

/// Result type for web3rs.
pub type Result<T> = std::result::Result<T, Error>;
