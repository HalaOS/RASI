use uint::{FromDecStrErr, FromHexError};

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

    
}

/// Result type for web3rs.
pub type Result<T> = std::result::Result<T, Error>;
