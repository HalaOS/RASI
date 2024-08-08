//! Error types and utilities.

use std::num::ParseIntError;

use crate::{
    eip::eip712::serde::{EncodeDataError, EncodeTypeError, TypeDefinitionError},
    prelude::Address,
    primitives::{balance::ParseBalanceError, HexError},
};

#[cfg(feature = "abi")]
use crate::abi::{AbiDeError, AbiSerError};

#[cfg(feature = "rlp")]
use crate::rlp::RlpError;

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

    #[cfg(feature = "rlp")]
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

    #[cfg(feature = "abi")]
    #[error(transparent)]
    ContractAbiSerError(#[from] AbiSerError),

    #[cfg(feature = "abi")]
    #[error(transparent)]
    ContractAbiDeError(#[from] AbiDeError),

    #[error("create logs filter failed: {0}")]
    FilterBuilder(String),

    #[error("signer accout list is empty")]
    SignerAccounts,

    #[error("signer accout not found: {0}")]
    SignerAccount(Address),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[cfg(feature = "wallet")]
    #[error(transparent)]
    SignatureError(#[from] ecdsa::signature::Error),

    #[cfg(feature = "wallet")]
    #[error("Invalid recovery id({0})")]
    InvalidRecoveryId(u8),

    #[cfg(feature = "wallet")]
    #[error(transparent)]
    EcError(#[from] elliptic_curve::Error),

    #[cfg(feature = "wallet")]
    #[error(transparent)]
    Bip32Error(#[from] crate::wallet::hdwallet::bip32::Bip32Error),
}

/// Result type for web3rs.
pub type Result<T> = std::result::Result<T, Error>;
