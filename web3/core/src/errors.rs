use std::num::ParseIntError;

use crate::{
    eip::eip712::serde::{EncodeDataError, EncodeTypeError, TypeDefinitionError},
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

    #[error(transparent)]
    Eip712EncodeDataError(#[from] EncodeDataError),

    #[error(transparent)]
    Eip712EncodeTypeError(#[from] EncodeTypeError),

    #[error(transparent)]
    Eip712TypeDefinitionError(#[from] TypeDefinitionError),
}

/// Result type for web3rs.
pub type Result<T> = std::result::Result<T, Error>;
