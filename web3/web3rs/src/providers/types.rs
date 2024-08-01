use ethnum::U256;
use serde::{Deserialize, Serialize};

use crate::primitives::{address::Address, h256::H256, hex::Hex};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AccessList(pub Vec<Access>);

impl Serialize for AccessList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AccessList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let accesses = Vec::<Access>::deserialize(deserializer)?;

        Ok(Self(accesses))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Access {
    pub address: Address,

    pub storage_keys: Vec<H256>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransactionOrHash {
    Null,
    Hash(H256),
    Transaction(Transaction),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    // 0x00
    #[serde(rename = "0x00")]
    Legacy,
    // 0x01
    #[serde(rename = "0x01")]
    Eip2930,
    // 0x02
    #[serde(rename = "0x02")]
    Eip1559,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    /// transaction type
    ///
    /// 1. Legacy (pre-EIP2718) `0x00`
    /// 2. EIP2930 (state access lists) `0x01`
    /// 3. EIP1559 0x02
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<TransactionType>,
    /// transaction nonce
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<U256>,
    /// To address
    pub to: Address,
    /// Gas limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<U256>,

    /// Transaction index in block
    #[serde(rename = "transactionIndex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction_index: Option<U256>,
    /// Block hash
    #[serde(rename = "blockHash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<H256>,
    /// Block number
    #[serde(rename = "blockNumber")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<U256>,
    /// Gas limit
    #[serde(rename = "gasPrice")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U256>,
    /// Transaction hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<H256>,
    /// Transfer eth value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// Input data to call contract.
    pub input: Hex<Vec<u8>>,
    /// Maximum fee per gas the sender is willing to pay to miners in wei
    #[serde(rename = "maxPriorityFeePerGas")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<U256>,
    /// Maximum total fee per gas the sender is willing to
    /// pay(includes the network/base fee and miner/ priority fee) in wei
    #[serde(rename = "maxFeePerGas")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<U256>,
    /// EIP-2930 access list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_list: Option<AccessList>,
    /// Chain ID tha this transaction is valid on
    #[serde(rename = "chainId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<U256>,
    /// The parity(0 for even, 1 for odd) of the y-value of the secp256k1 signature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<U256>,
    /// r-value of the secp256k1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<U256>,
    /// s-value of the secp256k1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<U256>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// Current block hash value
    pub hash: Option<H256>,
    /// Parent block hash
    #[serde(rename = "parentHash")]
    pub parent_hash: H256,

    /// Ommers hash
    #[serde(rename = "sha3Uncles")]
    pub sha3_uncles: Option<H256>,

    /// Coinbase
    pub miner: Address,

    /// State root
    #[serde(rename = "stateRoot")]
    pub state_root: H256,

    /// Transactions root
    #[serde(rename = "transactionsRoot")]
    pub transactions_root: H256,

    /// Receipts root
    #[serde(rename = "receiptsRoot")]
    pub receipts_root: H256,

    /// Bloom filter
    #[serde(rename = "logsBloom")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs_bloom: Option<Hex<Vec<u8>>>,

    /// Difficulty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<Hex<Vec<u8>>>,

    /// U256
    pub number: Option<U256>,

    /// Gas limit

    #[serde(rename = "gasLimit")]
    pub gas_limit: U256,

    /// Gas used
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,

    /// Timestamp
    pub timestamp: U256,

    /// Extra data
    #[serde(rename = "extraData")]
    pub extra_data: Hex<Vec<u8>>,

    /// Mix hash
    #[serde(rename = "mixHash")]
    pub mix_hash: Option<H256>,

    /// Nonce
    pub nonce: Option<U256>,

    /// Total difficult
    #[serde(rename = "totalDeffficult")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_deffficult: Option<Hex<Vec<u8>>>,

    /// Base fee per gas
    #[serde(rename = "baseFeePerGas")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_fee_per_gas: Option<U256>,

    /// Block size
    pub size: U256,

    /// transactions
    pub transactions: Vec<TransactionOrHash>,

    /// Uncles
    pub uncles: Vec<H256>,
}
