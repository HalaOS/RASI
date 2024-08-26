//! A Provider is an abstraction of a connection to the Ethereum network,
//! providing a concise, consistent interface to standard Ethereum node
//! functionality.

use std::{fmt::Debug, sync::Arc};

use crate::{
    eip::eip2718::TypedTransactionRequest,
    errors::{Error, Result},
    primitives::{Address, Bytes, H256, U256},
};

use async_trait::async_trait;
use futures_jsonrpcv2::client::JsonRpcClient;
use reweb3_num::cast::As;

use super::{
    Block, BlockNumberOrTag, Client, FeeHistory, Filter, FilterEvents, SyncingStatus, Transaction,
    TransactionReceipt,
};

fn map_error<E: Debug>(error: E) -> Error {
    Error::Other(format!("{:?}", error))
}

/// The JSON-RPC API is a popular method for interacting with Ethereum and is
/// available in all major Ethereum node implementations (e.g. Geth and Parity)
/// as well as many third-party web services (e.g. INFURA)
#[derive(Clone)]
pub struct JsonRpcProvider(
    /// jsonrpc2.0 client for this provider.
    Arc<JsonRpcClient>,
);

impl From<JsonRpcClient> for JsonRpcProvider {
    fn from(value: JsonRpcClient) -> Self {
        Self(Arc::new(value))
    }
}

#[async_trait]
impl Client for JsonRpcProvider {
    /// Returns the number of most recent block.
    async fn eth_blocknumber(&self) -> Result<U256> {
        Ok(self
            .0
            .clone()
            .call("eth_blockNumber", Vec::<String>::new())
            .await?)
    }

    /// Returns the chain ID of the current network
    async fn eth_chainid(&self) -> Result<u64> {
        let value: U256 = self.0.clone().call("eth_chainId", ()).await?;

        Ok(value.as_())
    }

    /// Returns information about a block by hash.
    async fn eth_getblockbyhash<H>(&self, hash: H, hydrated: bool) -> Result<Option<Block>>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send,
    {
        let hash: H256 = hash.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call("eth_getBlockByHash", (hash.to_string(), hydrated))
            .await?)
    }
    /// Returns information about a block by number.
    async fn eth_getblockbynumber<N>(
        &self,
        number_or_tag: N,
        hydrated: bool,
    ) -> Result<Option<Block>>
    where
        N: TryInto<BlockNumberOrTag> + Send,
        N::Error: Debug + Send,
    {
        let number_or_tag: BlockNumberOrTag = number_or_tag.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call("eth_getBlockByNumber", (number_or_tag, hydrated))
            .await?)
    }

    /// Returns transaction count number of block by block hash.
    async fn eth_get_block_transaction_count_by_hash<H>(&self, hash: H) -> Result<u64>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send,
    {
        let hash: H256 = hash.try_into().map_err(map_error)?;

        let value: U256 = self
            .0
            .clone()
            .call("eth_getBlockTransactionCountByHash", vec![hash])
            .await?;

        Ok(value.as_())
    }

    /// Returns the number of uncles in a block from a block matching the given block hash
    async fn eth_get_uncle_count_by_block_hash<H>(&self, hash: H) -> Result<u64>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send,
    {
        let hash: H256 = hash.try_into().map_err(map_error)?;

        let value: U256 = self
            .0
            .clone()
            .call("eth_getUncleCountByBlockHash", vec![hash])
            .await?;

        Ok(value.as_())
    }

    /// Returns the number of uncles in a block from a block matching the given block hash
    async fn eth_get_uncle_count_by_block_number<N>(&self, number_or_tag: N) -> Result<u64>
    where
        N: TryInto<BlockNumberOrTag> + Send,
        N::Error: Debug + Send,
    {
        let number_or_tag: BlockNumberOrTag = number_or_tag.try_into().map_err(map_error)?;

        let value: U256 = self
            .0
            .clone()
            .call("eth_getUncleCountByBlockNumber", vec![number_or_tag])
            .await?;

        Ok(value.as_())
    }

    /// Returns an object with data about the sync status or false
    async fn eth_syncing(&self) -> Result<SyncingStatus> {
        Ok(self
            .0
            .clone()
            .call("eth_syncing", Vec::<String>::new())
            .await?)
    }

    /// Returns the client coinbase address.
    async fn eth_coinbase(&self) -> Result<Address> {
        Ok(self
            .0
            .clone()
            .call("eth_coinbase", Vec::<String>::new())
            .await?)
    }

    /// Returns a list of addresses owned by client.
    async fn eth_accounts(&self) -> Result<Vec<Address>> {
        Ok(self
            .0
            .clone()
            .call("eth_accounts", Vec::<String>::new())
            .await?)
    }

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
        BT::Error: Debug + Send,
    {
        let transaction = transaction.try_into().map_err(map_error)?;

        if let Some(block_number_or_tag) = block_number_or_tag {
            let block_number_or_tag = block_number_or_tag.try_into().map_err(map_error)?;

            Ok(self
                .0
                .clone()
                .call("eth_call", (transaction, block_number_or_tag))
                .await?)
        } else {
            Ok(self.0.clone().call("eth_call", vec![transaction]).await?)
        }
    }

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
        BT::Error: Debug + Send,
    {
        let transaction = transaction.try_into().map_err(map_error)?;

        if let Some(block_number_or_tag) = block_number_or_tag {
            let block_number_or_tag = block_number_or_tag.try_into().map_err(map_error)?;

            Ok(self
                .0
                .clone()
                .call("eth_estimateGas", (transaction, block_number_or_tag))
                .await?)
        } else {
            Ok(self
                .0
                .clone()
                .call("eth_estimateGas", vec![transaction])
                .await?)
        }
    }

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
        BT::Error: Debug + Send,
    {
        let transaction = transaction.try_into().map_err(map_error)?;

        if let Some(block_number_or_tag) = block_number_or_tag {
            let block_number_or_tag = block_number_or_tag.try_into().map_err(map_error)?;

            Ok(self
                .0
                .clone()
                .call("eth_createAccessList", (transaction, block_number_or_tag))
                .await?)
        } else {
            Ok(self
                .0
                .clone()
                .call("eth_createAccessList", vec![transaction])
                .await?)
        }
    }

    /// Returns the current price gas in wei.
    async fn eth_gas_price(&self) -> Result<U256> {
        Ok(self
            .0
            .clone()
            .call("eth_gasPrice", Vec::<String>::new())
            .await?)
    }

    /// Returns the current maxPriorityFeePerGas per gas in wei.
    async fn eth_max_priority_fee_per_gas(&self) -> Result<U256> {
        Ok(self
            .0
            .clone()
            .call("eth_maxPriorityFeePerGas", Vec::<String>::new())
            .await?)
    }

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
        RP: AsRef<[f64]> + Send,
    {
        let block_count = block_count.try_into().map_err(map_error)?;

        let newest_block = newest_block.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call(
                "eth_feeHistory",
                (block_count, newest_block, reward_percentiles.as_ref()),
            )
            .await?)
    }

    /// Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.
    async fn eth_new_filter<F>(&self, filter: F) -> Result<U256>
    where
        F: TryInto<Filter> + Send,
        F::Error: Debug + Send,
    {
        let filter = filter.try_into().map_err(map_error)?;

        Ok(self.0.clone().call("eth_newFilter", vec![filter]).await?)
    }

    /// Creates new filter in the node,to notify when a new block arrives.
    async fn eth_new_block_filter(&self) -> Result<U256> {
        Ok(self
            .0
            .clone()
            .call("eth_newBlockFilter", Vec::<String>::new())
            .await?)
    }

    /// Creates new filter in the node,to notify when new pending transactions arrive.
    async fn eth_new_pending_transaction_filter(&self) -> Result<U256> {
        Ok(self
            .0
            .clone()
            .call("eth_newPendingTransactionFilter", Vec::<String>::new())
            .await?)
    }

    /// Uninstalls a filter with given id
    async fn eth_uninstall_filter<N>(&self, id: N) -> Result<bool>
    where
        N: TryInto<U256> + Send,
        N::Error: Debug + Send,
    {
        let id = id.try_into().map_err(map_error)?;

        Ok(self.0.clone().call("eth_uninstallFilter", vec![id]).await?)
    }

    /// Polling method for a filter, which returns an arrya of logs which occurred since last poll

    async fn eth_get_filter_changes<N>(&self, id: N) -> Result<Option<FilterEvents>>
    where
        N: TryInto<U256> + Send,
        N::Error: Debug + Send,
    {
        let id: U256 = id.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call("eth_getFilterChanges", vec![id])
            .await?)
    }

    /// Returns any arrays of all logs matching filter with given id
    async fn eth_get_filter_logs<N>(&self, id: N) -> Result<FilterEvents>
    where
        N: TryInto<U256> + Send,
        N::Error: Debug + Send,
    {
        let id = id.try_into().map_err(map_error)?;

        Ok(self.0.clone().call("eth_getFilterLogs", vec![id]).await?)
    }

    /// Returns an array of all logs matching filter with filter description
    async fn eth_get_logs<F>(&self, filter: F) -> Result<FilterEvents>
    where
        F: TryInto<Filter> + Send,
        F::Error: Debug + Send,
    {
        let filter = filter.try_into().map_err(map_error)?;

        Ok(self.0.clone().call("eth_getLogs", vec![filter]).await?)
    }

    /// Returns an RLP encoded transaction signed by the specified account.
    async fn eth_sign_transaction<T>(&self, transaction: T) -> Result<Bytes>
    where
        T: TryInto<Transaction> + Send,
        T::Error: Debug + Send,
    {
        let transaction = transaction.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call("eth_signTransaction", vec![transaction])
            .await?)
    }

    /// Returns the balance of the account given address.
    async fn eth_get_balance<A>(&self, address: A) -> Result<U256>
    where
        A: TryInto<Address> + Send,
        A::Error: Debug + Send,
    {
        let address = address.try_into().map_err(map_error)?;

        Ok(self.0.clone().call("eth_getBalance", vec![address]).await?)
    }

    /// Returns the number of transactions sent from an address
    async fn eth_get_transaction_count<A>(&self, address: A) -> Result<U256>
    where
        A: TryInto<Address> + Send,
        A::Error: Debug + Send,
    {
        let address = address.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call("eth_getTransactionCount", vec![address])
            .await?)
    }

    /// Submit a raw transaction.
    async fn eth_send_raw_transaction<B>(&self, raw: B) -> Result<H256>
    where
        B: TryInto<Bytes> + Send,
        B::Error: Debug + Send,
    {
        let raw = raw.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call("eth_sendRawTransaction", vec![raw])
            .await?)
    }

    async fn eth_get_transaction_by_hash<H>(&self, tx_hash: H) -> Result<Option<Transaction>>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send,
    {
        let tx_hash = tx_hash.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call("eth_getTransactionByHash", vec![tx_hash])
            .await?)
    }

    /// Returns the receipt of a transaction by transaction hash
    async fn eth_get_transaction_receipt<H>(&self, tx_hash: H) -> Result<Option<TransactionReceipt>>
    where
        H: TryInto<H256> + Send,
        H::Error: Debug + Send,
    {
        let tx_hash = tx_hash.try_into().map_err(map_error)?;

        Ok(self
            .0
            .clone()
            .call("eth_getTransactionReceipt", vec![tx_hash])
            .await?)
    }
}

#[cfg(all(test, feature = "test-clients"))]
mod tests {
    use std::{
        str::FromStr,
        sync::{Once, OnceLock},
    };

    use futures_jsonrpcv2::rasi::http::HttpJsonRpcClient;
    use rasi_mio::{net::register_mio_network, timer::register_mio_timer};

    use crate::{
        abi::from_abi,
        clients::{BlockTag, Topic},
        prelude::keccak256,
    };

    use super::*;

    fn init() -> JsonRpcProvider {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            // pretty_env_logger::init_timed();
            register_mio_network();
            register_mio_timer();
            pretty_env_logger::init();
        });

        static CLIENT: OnceLock<JsonRpcProvider> = OnceLock::new();

        CLIENT
            .get_or_init(|| {
                HttpJsonRpcClient::new("https://eth-mainnet.token.im")
                    // only for some old servers.
                    // .set_use_server_name_indication(false)
                    .create()
                    .unwrap()
                    .into()
            })
            .clone()
    }

    #[futures_test::test]
    async fn test_eth_block_number() {
        let provider = init();

        provider.eth_blocknumber().await.unwrap();
    }

    #[futures_test::test]
    async fn test_eth_chain_id() {
        let provider = init();

        provider.eth_chainid().await.unwrap();
    }

    #[futures_test::test]
    async fn eth_getblockbyhash() {
        let provider = init();

        let block = provider
            .eth_getblockbyhash(
                "0x0ade5e5a2ca4fbc215fe6fcca10bf198e24410da129ff0dc375391ce5fccc309",
                false,
            )
            .await
            .unwrap();

        println!("{}", serde_json::to_string(&block).unwrap());
    }

    #[futures_test::test]
    async fn eth_getblockbynumber() {
        let provider = init();

        let _ = provider.eth_getblockbynumber("0x00", false).await.unwrap();

        let _ = provider
            .eth_getblockbynumber(BlockTag::Finalized, false)
            .await
            .unwrap();
    }

    #[futures_test::test]
    async fn eth_get_block_transaction_count_by_hash() {
        let provider = init();

        let block = provider
            .eth_getblockbynumber(BlockTag::Latest, false)
            .await
            .unwrap()
            .unwrap();

        let hash = block.hash.unwrap();

        let _ = provider.eth_getblockbyhash(hash, false).await.unwrap();
    }

    #[futures_test::test]
    async fn eth_get_uncle_count_by_block_hash() {
        let provider = init();

        let block = provider
            .eth_getblockbynumber(BlockTag::Latest, false)
            .await
            .unwrap()
            .unwrap();

        let hash = block.hash.unwrap();

        let _ = provider
            .eth_get_uncle_count_by_block_hash(hash)
            .await
            .unwrap();
    }

    #[futures_test::test]
    async fn eth_get_uncle_count_by_block_number() {
        let provider = init();

        let _ = provider
            .eth_get_uncle_count_by_block_number("0x00")
            .await
            .unwrap();

        let _ = provider
            .eth_get_uncle_count_by_block_number(BlockTag::Finalized)
            .await
            .unwrap();
    }

    #[futures_test::test]
    async fn eth_get_logs() {
        let provider = init();

        let filter = Filter::new()
            .with_block(20427505)
            .with_address("0x1425273B6696d9f8Dbbe60FE4340029548590a45")
            .with_topics((
                Topic::from([
                    keccak256("Transfer(address,address,uint256)"),
                    keccak256("ETHReceived1(uint256)"),
                ]),
                Topic::from_simple_value(
                    Address::from_str("0x9ffaBEC0355d7ae65E3441d02b9eF212cF3A943F").unwrap(),
                ),
            ))
            .create()
            .unwrap();

        let events = provider.eth_get_logs(filter).await.unwrap();

        if let FilterEvents::Logs(logs) = events {
            assert_eq!(logs.len(), 1);

            let amount: U256 = from_abi(&logs[0].data).unwrap();

            assert_eq!(amount, U256::from(7033000000000000000000u128));
        } else {
            assert!(false);
        }
    }
}
