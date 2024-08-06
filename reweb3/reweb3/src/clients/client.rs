use std::fmt::Debug;

use crate::{
    eip::eip2718::TypedTransactionRequest,
    errors::Result,
    primitives::{Address, Bytes, H256, U256},
};

use async_trait::async_trait;

use super::{
    Block, BlockNumberOrTag, FeeHistory, Filter, FilterEvents, SyncingStatus, Transaction,
    TransactionReceipt,
};

/// This trait represents the async ethereum client.
#[async_trait]
pub trait Client {
    /// Returns the number of most recent block.
    async fn eth_blocknumber(&self) -> Result<U256>;

    /// Returns the chain ID of the current network
    async fn eth_chainid(&self) -> Result<u64>;

    /// Returns information about a block by hash.
    async fn eth_getblockbyhash<H>(&self, hash: H, hydrated: bool) -> Result<Option<Block>>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send;
    /// Returns information about a block by number.
    async fn eth_getblockbynumber<N>(
        &self,
        number_or_tag: N,
        hydrated: bool,
    ) -> Result<Option<Block>>
    where
        N: TryInto<BlockNumberOrTag> + Send,
        N::Error: Debug + Send;

    /// Returns transaction count number of block by block hash.
    async fn eth_get_block_transaction_count_by_hash<H>(&self, hash: H) -> Result<u64>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send;

    /// Returns the number of uncles in a block from a block matching the given block hash
    async fn eth_get_uncle_count_by_block_hash<H>(&self, hash: H) -> Result<u64>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send;

    /// Returns the number of uncles in a block from a block matching the given block hash
    async fn eth_get_uncle_count_by_block_number<N>(&self, number_or_tag: N) -> Result<u64>
    where
        N: TryInto<BlockNumberOrTag> + Send,
        N::Error: Debug + Send;

    /// Returns an object with data about the sync status or false
    async fn eth_syncing(&self) -> Result<SyncingStatus>;

    /// Returns the client coinbase address.
    async fn eth_coinbase(&self) -> Result<Address>;

    /// Returns a list of addresses owned by client.
    async fn eth_accounts(&self) -> Result<Vec<Address>>;

    /// Executes a new message call immediately without creating a transaction on the block chain.
    async fn eth_call<TX, BT>(
        &self,
        transaction: TX,
        block_number_or_tag: Option<BT>,
    ) -> Result<Bytes>
    where
        TX: TryInto<TypedTransactionRequest> + Send,
        TX::Error: Debug + Send,
        BT: TryInto<BlockNumberOrTag> + Send,
        BT::Error: Debug + Send;

    /// Generates and returns an estimate of how much gas is necessary to allow the transaction to complete.
    async fn eth_estimate_gas<TX, BT>(
        &self,
        transaction: TX,
        block_number_or_tag: Option<BT>,
    ) -> Result<U256>
    where
        TX: TryInto<TypedTransactionRequest> + Send,
        TX::Error: Debug + Send,
        BT: TryInto<BlockNumberOrTag> + Send,
        BT::Error: Debug + Send;

    /// Generates an access list for a transaction
    async fn eth_create_accesslist<TX, BT>(
        &self,
        transaction: TX,
        block_number_or_tag: Option<BT>,
    ) -> Result<U256>
    where
        TX: TryInto<Transaction> + Send,
        TX::Error: Debug + Send,
        BT: TryInto<BlockNumberOrTag> + Send,
        BT::Error: Debug + Send;

    /// Returns the current price gas in wei.
    async fn eth_gas_price(&self) -> Result<U256>;

    /// Returns the current maxPriorityFeePerGas per gas in wei.
    async fn eth_max_priority_fee_per_gas(&self) -> Result<U256>;

    /// Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.
    async fn eth_fee_history<N, BT, RP>(
        &self,
        block_count: N,
        newest_block: BT,
        reward_percentiles: RP,
    ) -> Result<FeeHistory>
    where
        N: TryInto<U256> + Send,
        N::Error: Debug + Send,
        BT: TryInto<BlockNumberOrTag> + Send,
        BT::Error: Debug + Send,
        RP: AsRef<[f64]> + Send;

    /// Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.
    async fn eth_new_filter<F>(&self, filter: F) -> Result<U256>
    where
        F: TryInto<Filter> + Send,
        F::Error: Debug + Send;

    /// Creates new filter in the node,to notify when a new block arrives.
    async fn eth_new_block_filter(&self) -> Result<U256>;

    /// Creates new filter in the node,to notify when new pending transactions arrive.
    async fn eth_new_pending_transaction_filter(&self) -> Result<U256>;

    /// Uninstalls a filter with given id
    async fn eth_uninstall_filter<N>(&self, id: N) -> Result<bool>
    where
        N: TryInto<U256> + Send,
        N::Error: Debug + Send;

    /// Polling method for a filter, which returns an arrya of logs which occurred since last poll

    async fn eth_get_filter_changes<N>(&self, id: N) -> Result<Option<FilterEvents>>
    where
        N: TryInto<U256> + Send,
        N::Error: Debug + Send;

    /// Returns any arrays of all logs matching filter with given id
    async fn eth_get_filter_logs<N>(&self, id: N) -> Result<FilterEvents>
    where
        N: TryInto<U256> + Send,
        N::Error: Debug + Send;

    /// Returns an array of all logs matching filter with filter description
    async fn eth_get_logs<F>(&self, filter: F) -> Result<FilterEvents>
    where
        F: TryInto<Filter> + Send,
        F::Error: Debug + Send;

    /// Returns an RLP encoded transaction signed by the specified account.
    async fn eth_sign_transaction<T>(&self, transaction: T) -> Result<Bytes>
    where
        T: TryInto<Transaction> + Send,
        T::Error: Debug + Send;

    /// Returns the balance of the account given address.
    async fn eth_get_balance<A>(&self, address: A) -> Result<U256>
    where
        A: TryInto<Address> + Send,
        A::Error: Debug + Send;

    /// Returns the number of transactions sent from an address
    async fn eth_get_transaction_count<A>(&self, address: A) -> Result<U256>
    where
        A: TryInto<Address> + Send,
        A::Error: Debug + Send;

    /// Submit a raw transaction.
    async fn eth_send_raw_transaction<B>(&self, raw: B) -> Result<H256>
    where
        B: TryInto<Bytes> + Send,
        B::Error: Debug + Send;

    async fn eth_get_transaction_by_hash<H>(&self, tx_hash: H) -> Result<Option<Transaction>>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send;

    /// Returns the receipt of a transaction by transaction hash
    async fn eth_get_transaction_receipt<H>(
        &self,
        tx_hash: H,
    ) -> Result<Option<TransactionReceipt>>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send;

    #[allow(unused)]
    async fn call(
        &self,
        signature: &str,
        contract_address: &Address,
        call_data: Vec<u8>,
    ) -> Result<Vec<u8>> {
        todo!()
    }
}
