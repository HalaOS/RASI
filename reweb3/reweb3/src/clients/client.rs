use std::{fmt::Debug, time::Duration};

use crate::{
    eip::eip2718::{LegacyTransactionRequest, TypedTransactionRequest},
    errors::{Error, Result},
    prelude::keccak256,
    primitives::{Address, Bytes, H256, U256},
};

use async_trait::async_trait;
use reweb3_num::cast::As;

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

    /// Wait for the transaction to complete or time out.
    ///
    /// # Timeout
    ///
    /// If the transaction is not found in `5` consecutive mined blocks,
    /// the function call returns a timeout error.
    ///
    /// # configurable
    ///
    /// If you need precise control over the whole process, use [`transaction_wait_with`](ClientExt::transaction_wait_with) instead of.
    #[cfg(feature = "rasi")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rasi")))]
    async fn transaction_wait(&self, tx_hash: H256) -> Result<TransactionReceipt>
    where
        Self: Sized,
    {
        use rasi::timer::sleep;

        let mut wait = TransactionWait::new(tx_hash, self).await?;

        loop {
            if let Some(receipt) = wait.poll_once().await? {
                return Ok(receipt);
            }

            sleep(wait.timeout()).await;
        }
    }

    /// Plus version of [`transaction_wait`](ClientExt::transaction_wait)
    #[cfg(feature = "rasi")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rasi")))]
    async fn transaction_wait_with<F>(
        &self,
        builder: TransactionWaitBuilder,
    ) -> Result<TransactionReceipt>
    where
        Self: Sized,
    {
        use rasi::timer::sleep;

        let mut wait = builder.create(self).await?;

        loop {
            if let Some(receipt) = wait.poll_once().await? {
                return Ok(receipt);
            }

            sleep(wait.timeout()).await;
        }
    }
}

/// A builder pattern implementation for [`TransactionWait`]
pub struct TransactionWaitBuilder {
    tx_hash: H256,
    max_sleep_timeout: Duration,
    blocks: U256,
}

impl TransactionWaitBuilder {
    fn new(tx_hash: H256) -> Self {
        Self {
            tx_hash,
            max_sleep_timeout: Duration::from_secs(300),
            blocks: 5usize.into(),
        }
    }

    /// Set the `max_sleep_timeout` flag.
    /// generally this value must be equal to time interval of blockchain's one epoch.
    pub fn set_max_sleep_timeout(mut self, timeout: Duration) -> Self {
        self.max_sleep_timeout = timeout;
        self
    }

    /// Set the `max_waiting_blocks` value. the default value is `2`.
    ///
    /// This flag value control the maximun waiting blocks for one transaction.
    pub fn set_max_waiting_blocks(mut self, blocks: usize) -> Self {
        self.blocks = blocks.into();
        self
    }

    /// Create a new `TransactionWait` instance with this configuration.
    pub async fn create<'a, C>(self, client: &'a C) -> Result<TransactionWait<'a, C>>
    where
        C: Client,
    {
        let TransactionWaitBuilder {
            tx_hash,
            max_sleep_timeout,
            blocks,
        } = self;

        let block_num: U256 = client.eth_blocknumber().await?.into();

        let (duration, suggested) = if block_num > 0usize.into() {
            let older_block_num = block_num - U256::from(1usize);

            let older_block = client
                .eth_getblockbynumber(older_block_num, false)
                .await?
                .ok_or(Error::Other(format!(
                    "fetch block({}), returns None",
                    older_block_num
                )))?;

            let block = client
                .eth_getblockbynumber(older_block_num, false)
                .await?
                .ok_or(Error::Other(format!(
                    "fetch block({}), returns None",
                    older_block_num
                )))?;

            (
                Duration::from_secs((block.timestamp - older_block.timestamp).as_()) / 2,
                true,
            )
        } else {
            (Duration::from_secs(1), false)
        };

        Ok(TransactionWait {
            poll_sleep_timeout: duration,
            poll_sleep_time_is_suggested: suggested,
            provider: client,
            to_block: block_num + blocks,
            tx_hash,
            max_sleep_timeout,
        })
    }
}

/// A poller for transaction receipt.
pub struct TransactionWait<'a, C> {
    tx_hash: H256,
    max_sleep_timeout: Duration,
    poll_sleep_timeout: Duration,
    poll_sleep_time_is_suggested: bool,
    provider: &'a C,
    /// Maximum number of block to waiting for
    to_block: U256,
}

impl<'a, C> TransactionWait<'a, C>
where
    C: Client,
{
    /// Create a new builder for `TransactionWait`
    pub fn with_builder(tx_hash: H256) -> TransactionWaitBuilder {
        TransactionWaitBuilder::new(tx_hash)
    }

    /// Create a new `TransactionWait` instance with default configuration.
    pub async fn new(tx_hash: H256, client: &'a C) -> Result<Self> {
        TransactionWaitBuilder::new(tx_hash).create(client).await
    }

    /// Call `eth_get_transaction_receipt` once.
    ///
    /// If this function returns `None`,you should call `on_timeout` to get the recommended
    /// sleeping time before the next time this function
    pub async fn poll_once(&mut self) -> Result<Option<TransactionReceipt>> {
        let reciept = self
            .provider
            .eth_get_transaction_receipt(self.tx_hash.clone())
            .await?;

        if reciept.is_none()
            && !self.poll_sleep_time_is_suggested
            && self.poll_sleep_timeout < self.max_sleep_timeout
        {
            let block_num: U256 = self.provider.eth_blocknumber().await?.into();
            if block_num > self.to_block {
                return Err(Error::Other(format!(
                    "TransactionWaiter: reach maximun waiting blocks"
                )));
            }

            self.poll_sleep_timeout *= 2;

            if self.poll_sleep_timeout > self.max_sleep_timeout {
                self.poll_sleep_timeout = self.max_sleep_timeout;
            }
        }

        Ok(reciept)
    }

    /// The recommended sleeping time before the next time this calling function [`poll_once`](Self::poll_once)
    pub fn timeout(&self) -> Duration {
        self.poll_sleep_timeout.clone()
    }
}
