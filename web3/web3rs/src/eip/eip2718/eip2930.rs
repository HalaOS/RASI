use serde::{Deserialize, Serialize};

use crate::{
    errors::Result,
    primitives::{Bytes, Eip1559Signature, H256},
};

use super::{AccessList, LegacyTransactionRequest};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Eip2930TransactionRequest {
    #[serde(flatten)]
    pub tx: LegacyTransactionRequest,

    pub access_list: AccessList,
}

impl Eip2930TransactionRequest {
    /// Generate legacy transaction sign hash.
    pub fn sign_hash(&self) -> Result<H256> {
        todo!()
    }

    pub fn rlp(&self) -> Result<Bytes> {
        todo!()
    }

    /// Returns signed tx rlp encoding stream.
    pub fn rlp_signed(&self, signature: Eip1559Signature) -> Result<Bytes> {
        todo!()
    }
}
