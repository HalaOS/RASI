//! This module implements the transaction types and signature methods
//! required by [`eip1559`](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1559.md),
//! [`eip2930`](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-2930.md) and `Legacy`.
//!
//! The signature method is only valid when the `rlp` feature is enabled.

#[cfg(feature = "rlp")]
use crate::{
    errors::Result,
    primitives::{Bytes, Eip1559Signature, H256},
};

use serde::{Deserialize, Serialize};

/// Represents the various request types for Ethernet transactions.
/// The jsonrpc api uses it to deserialize transactions..
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum TypedTransactionRequest {
    // 0x00
    #[serde(rename = "0x00")]
    Legacy(LegacyTransactionRequest),
    // 0x01
    #[serde(rename = "0x01")]
    Eip2930(Eip2930TransactionRequest),
    // 0x02
    #[serde(rename = "0x02")]
    Eip1559(Eip1559TransactionRequest),
}

impl From<LegacyTransactionRequest> for TypedTransactionRequest {
    fn from(tx: LegacyTransactionRequest) -> Self {
        TypedTransactionRequest::Legacy(tx)
    }
}

impl From<Eip2930TransactionRequest> for TypedTransactionRequest {
    fn from(tx: Eip2930TransactionRequest) -> Self {
        TypedTransactionRequest::Eip2930(tx)
    }
}

impl From<Eip1559TransactionRequest> for TypedTransactionRequest {
    fn from(tx: Eip1559TransactionRequest) -> Self {
        TypedTransactionRequest::Eip1559(tx)
    }
}

#[cfg(feature = "rlp")]
impl TypedTransactionRequest {
    pub fn sign_hash(&self) -> Result<H256> {
        match self {
            Self::Legacy(tx) => tx.sign_hash(),
            Self::Eip2930(tx) => tx.sign_hash(),
            Self::Eip1559(tx) => tx.sign_hash(),
        }
    }

    pub fn rlp_signed(&self, signature: Eip1559Signature) -> Result<Bytes> {
        match self {
            Self::Legacy(tx) => tx.rlp_signed(signature),
            Self::Eip2930(tx) => tx.rlp_signed(signature),
            Self::Eip1559(tx) => tx.rlp_signed(signature),
        }
    }
}

mod accesslist;
pub use accesslist::*;

mod legacy;
pub use legacy::*;

mod eip2930;
pub use eip2930::*;

mod eip1559;
pub use eip1559::*;
