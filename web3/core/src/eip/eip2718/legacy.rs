use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::primitives::{Address, Bytes, Eip1559Signature, H256, U256};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegacyTransactionRequest {
    /// Transaction nonce
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<U256>,
    /// Gas price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U256>,
    /// Supplied gas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<U256>,
    /// Recipient address (None for contract creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// Transferred value
    pub value: Option<U256>,
    /// The compiled code of a contract OR the first 4 bytes of the hash of the
    /// invoked method signature and encoded parameters. For details see Ethereum Contract ABI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
    /// Chain id for EIP-155
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<U256>,
}

impl LegacyTransactionRequest {
    /// Generate legacy transaction sign hash.
    pub fn sign_hash(&self) -> Result<H256> {
        todo!()
    }

    pub fn rlp(&self) -> Result<Bytes> {
        todo!()
    }

    /// Returns signed tx rlp encoding stream.
    pub fn rlp_signed(&self, _: Eip1559Signature) -> Result<Bytes> {
        todo!()
    }
}
