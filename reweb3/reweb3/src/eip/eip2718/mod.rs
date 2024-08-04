use crate::primitives::Address;

#[cfg(feature = "rlp")]
use crate::{
    errors::Result,
    primitives::{Bytes, Eip1559Signature, H256},
};

pub fn keccak256<S>(bytes: S) -> [u8; 32]
where
    S: AsRef<[u8]>,
{
    let mut hasher = Keccak256::new();

    hasher.update(bytes.as_ref());

    hasher.finalize().into()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NameOrAddress {
    Name(String),
    Address(Address),
}

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

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
