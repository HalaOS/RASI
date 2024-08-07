use std::{fmt::Debug, sync::OnceLock, time::Duration};

use crate::{
    eip::eip2718::{LegacyTransactionRequest, TypedTransactionRequest},
    errors::Result,
    primitives::{Address, Bytes, H256, U256},
    prelude::keccak256,
};

use async_trait::async_trait;
use dashmap::DashMap;

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
}

/// An extension trait for [`Client`]
#[async_trait]
pub trait ClientExt: Client {
    /// Call contract pure/view function.
    async fn call_contract(
        &self,
        signature: &str,
        contract_address: &Address,
        mut call_data: Bytes,
    ) -> Result<Bytes> {
        let mut selector_name = keccak256(signature.as_bytes()).as_ref()[0..4].to_vec();

        selector_name.append(&mut call_data.0);

        let call_data: Bytes = selector_name.into();

        let mut request = LegacyTransactionRequest::default();

        request.to = Some(contract_address.clone());

        request.data = Some(call_data);

        self.eth_call(request, None::<BlockNumberOrTag>).await
    }

    /// Wait a transaction to complete and returns `TransactionReceipt` on succeed.
    ///
    /// Returning [`Ok`] only means that the wait process was successful,
    /// You should check the returned [`TransactionReceipt`] for the status of the transaction.
    ///
    /// # Parameters
    ///
    /// - blocks: lookup to `blocks` blocks
    #[cfg(feature = "rasi")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rasi")))]
    async fn tx_wait(&self, hash: H256, blocks: Option<U256>) -> Result<TransactionReceipt> {
        use rasi::timer::sleep;
        use reweb3_num::cast::As;

        use crate::errors::Error;

        let blocks = blocks.unwrap_or(U256::from(2usize));

        if let Some(receipt) = self.eth_get_transaction_receipt(hash.clone()).await? {
            return Ok(receipt);
        }

        // get the chain id first
        let chain_id: U256 = self.eth_chainid().await?.into();

        let (mut duration, suggested) = if let Some(duration) = get_mine_interval(&chain_id) {
            (duration, true)
        } else {
            (Duration::from_secs(1), false)
        };

        let latest_block_number = self.eth_blocknumber().await?;

        loop {
            if let Some(receipt) = self.eth_get_transaction_receipt(hash.clone()).await? {
                if !suggested && receipt.block_number > 10usize.into() {
                    let block_number = receipt.block_number - U256::from(10usize);
                    let older_block = self
                        .eth_getblockbynumber(receipt.block_number - U256::from(10usize), false)
                        .await?
                        .ok_or(Error::Other(format!(
                            "fetch block, chain_id={:x}, num={:x}, error='not found'",
                            chain_id, block_number,
                        )))?;

                    let block_number = receipt.block_number;

                    let mine_block = self
                        .eth_getblockbynumber(receipt.block_number - U256::from(10usize), false)
                        .await?
                        .ok_or(Error::Other(format!(
                            "fetch block, chain_id={:x}, num={:x}, error='not found'",
                            chain_id, block_number,
                        )))?;

                    let duration = mine_block.timestamp - older_block.timestamp;

                    let suggest_duration = Duration::from_secs(duration.as_()) / 10;

                    set_mine_interval(chain_id, suggest_duration);
                }
                return Ok(receipt);
            }

            let block_number = self.eth_blocknumber().await?;

            if block_number - latest_block_number > blocks {
                return Err(Error::Other(format!(
                    "tx_wait, chain_id={:x}, error='Maximum number of lookup blocks reached({})'",
                    chain_id, blocks,
                )));
            }

            sleep(duration).await;

            if !suggested && duration < Duration::from_secs(300) {
                duration = duration * 2;
            }
        }
    }
}

#[cfg(feature = "rasi")]
#[cfg_attr(docsrs, doc(cfg(feature = "rasi")))]

mod tx_wait {
    use super::*;

    static CHAIN_MINE_INTERVALS: OnceLock<DashMap<U256, Duration>> = OnceLock::new();

    fn create_default_chain_mine_intervals() -> DashMap<U256, Duration> {
        DashMap::new()
    }

    pub(super) fn get_mine_interval(chain_id: &U256) -> Option<Duration> {
        CHAIN_MINE_INTERVALS
            .get_or_init(create_default_chain_mine_intervals)
            .get(chain_id)
            .map(|value| value.clone())
    }

    /// Set the suggestion mining interval of chain by `chain_id`.
    ///
    /// This function is useful for [`tx_wait`](ClientExt::tx_wait) function.
    pub fn set_mine_interval(chain_id: U256, duration: Duration) {
        CHAIN_MINE_INTERVALS
            .get_or_init(create_default_chain_mine_intervals)
            .insert(chain_id, duration);
    }
}
#[cfg(feature = "rasi")]
pub use tx_wait::*;
