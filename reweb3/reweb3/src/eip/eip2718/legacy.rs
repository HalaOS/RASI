use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::primitives::{Address, Bytes, Eip1559Signature, H256, U256};
use crate::rlp::RlpEncoder;

use super::keccak256;

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
        Ok(keccak256(self.rlp()?.0).into())
    }

    pub fn rlp(&self) -> Result<Bytes> {
        let mut s = RlpEncoder::default();

        let chain_id = self.chain_id.unwrap_or(U256::from(1u32));

        (
            &self.nonce,
            &self.gas_price,
            &self.gas,
            &self.to,
            &self.value,
            &self.data,
            chain_id,
            0x0u8,
            0x0u8,
        )
            .serialize(&mut s)?;

        Ok(s.finalize()?.into())
    }

    /// Returns signed tx rlp encoding stream.
    pub fn rlp_signed(&self, signature: Eip1559Signature) -> Result<Bytes> {
        let mut rlp = RlpEncoder::default();

        // encode v,r,s
        let chain_id = self.chain_id.unwrap_or(U256::from(1u32));

        let v: U256 = U256::from(signature.v) + 35 + chain_id * U256::from(2u32);

        (
            &self.nonce,
            &self.gas_price,
            &self.gas,
            &self.to,
            &self.value,
            &self.data,
            v,
            signature.r,
            signature.s,
        )
            .serialize(&mut rlp)?;

        Ok(rlp.finalize()?.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json::json;

    #[test]
    fn test_rlp() {
        let tx = json!({
            "chainId":"0x1",
            "nonce": "0x1",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "value":"0x1",
            "data":"0x",
            "gas":"0x60000",
            "gasPrice": "0x60000111"
        });

        let tx: LegacyTransactionRequest = serde_json::from_value(tx).unwrap();

        assert_eq!(
            tx.rlp().unwrap().to_string(),
            "0xe4018460000111830600009470997970c51812dc3a010c7d01b50e0d17dc79c80180018080"
        );

        let sig: Eip1559Signature = "0x016c7e1e13070e6f10e51d7d20e986c59fd080fc6afc5508f44e8b0a84a58b7d1a13c20fa2b6d77ae6814a41b674946387dde6401c73eb0cab2246a2981c48e344".parse().unwrap();

        assert_eq!(
            tx.rlp_signed(sig).unwrap().to_string(),
            "0xf864018460000111830600009470997970c51812dc3a010c7d01b50e0d17dc79c8018026a06c7e1e13070e6f10e51d7d20e986c59fd080fc6afc5508f44e8b0a84a58b7d1aa013c20fa2b6d77ae6814a41b674946387dde6401c73eb0cab2246a2981c48e344"
        );
    }

    #[test]
    fn test_rlp1() {
        let tx = json!({
            "nonce": "0x9",
            "to": "0x3535353535353535353535353535353535353535",
            "value":"0xDE0B6B3A7640000",
            "data":"0x",
            "gas":"0x5208",
            "gasPrice": "0x4A817C800"
        });

        let tx: LegacyTransactionRequest = serde_json::from_value(tx).unwrap();

        assert_eq!(
            tx.rlp().unwrap().to_string(),
            "0xec098504a817c800825208943535353535353535353535353535353535353535880de0b6b3a764000080018080"
        );
    }
}
