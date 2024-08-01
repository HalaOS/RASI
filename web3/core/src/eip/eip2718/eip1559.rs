use serde::{Deserialize, Serialize};

use super::{AccessList, H256};

use crate::{
    errors::Result,
    primitives::{Address, Bytes, Eip1559Signature, U256},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Eip1559TransactionRequest {
    pub chain_id: U256,

    /// Transaction nonce
    pub nonce: U256,
    /// Gas price
    pub max_priority_fee_per_gas: U256,

    pub max_fee_per_gas: U256,
    /// Supplied gas
    pub gas: U256,
    /// Recipient address (None for contract creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// Transferred value
    pub value: Option<U256>,
    /// The compiled code of a contract OR the first 4 bytes of the hash of the
    /// invoked method signature and encoded parameters. For details see Ethereum Contract ABI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,

    pub access_list: AccessList,
}

impl Eip1559TransactionRequest {
    /// Generate legacy transaction sign hash.
    pub fn sign_hash(&self) -> Result<H256> {
        todo!()
    }

    pub fn rlp(&self) -> Result<Bytes> {
        todo!()
    }

    /// Returns signed tx rlp encoding stream.
    pub fn rlp_signed(&self, _signature: Eip1559Signature) -> Result<Bytes> {
        todo!()
    }
}
