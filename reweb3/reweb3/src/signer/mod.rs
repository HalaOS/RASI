//! The module provides various implementations of ethereum signer

use std::fmt::Debug;

use async_trait::async_trait;
use serde::Serialize;

use crate::{
    eip::{eip2718::TypedTransactionRequest, eip712::TypedData},
    errors::Result,
    primitives::{Bytes, Eip1559Signature},
};

/// Represent a data signer for etherenum client.
#[async_trait]
pub trait Signer {
    /// Signs the transaction request and returns the signed transaction request in [`rlp`](crate::rlp) format.
    async fn sign_transaction<T>(&self, request: T) -> Result<Bytes>
    where
        T: TryInto<TypedTransactionRequest> + Send,
        T::Error: Debug + Send;

    /// Calculate the signature of the [`TypedData`] defined in [`eip-712`](https://eips.ethereum.org/EIPS/eip-712)
    async fn sign_typed_data<V>(&self, typed_data: TypedData<V>) -> Result<Eip1559Signature>
    where
        V: Serialize + Send;
}

#[cfg(feature = "clients")]
#[cfg_attr(docsrs, doc(cfg(feature = "clients")))]
mod with_client {

    use reweb3_num::types::U256;

    use crate::{
        clients::{
            Block, BlockNumberOrTag, Client, FeeHistory, Filter, FilterEvents, SyncingStatus,
            Transaction, TransactionReceipt,
        },
        primitives::{balance::TransferOptions, Address, Bytes, H256},
    };

    use super::*;

    /// A combinated trait of [`Signer`] and [`Client`]
    #[async_trait]
    pub trait SignerWithProvider: Signer + Client + Send + Sync {
        #[allow(unused)]
        async fn deploy(
            &self,
            bytecode: &str,
            signature: &str,
            call_data: Vec<u8>,
            ops: TransferOptions,
        ) -> Result<Address> {
            todo!()
        }

        #[allow(unused)]
        async fn sign_and_send_transaction(
            &self,
            signature: &str,
            contract_address: &Address,
            call_data: Vec<u8>,
            ops: TransferOptions,
        ) -> Result<H256> {
            todo!()
        }
    }

    #[async_trait]
    impl<S, C> Signer for (S, C)
    where
        S: Signer + Sync + Send + Unpin,
        C: Client + Sync + Send + Unpin,
    {
        /// Signs the transaction request and returns the signed transaction request in [`rlp`](crate::rlp) format.
        async fn sign_transaction<T>(&self, request: T) -> Result<Bytes>
        where
            T: TryInto<TypedTransactionRequest> + Send,
            T::Error: Debug + Send,
        {
            self.0.sign_transaction(request).await
        }

        /// Calculate the signature of the [`TypedData`] defined in [`eip-712`](https://eips.ethereum.org/EIPS/eip-712)
        async fn sign_typed_data<V>(&self, typed_data: TypedData<V>) -> Result<Eip1559Signature>
        where
            V: Serialize + Send,
        {
            self.0.sign_typed_data(typed_data).await
        }
    }

    #[async_trait]
    impl<S, C> Client for (S, C)
    where
        S: Signer + Sync + Send + Unpin,
        C: Client + Sync + Send + Unpin,
    {
        /// Returns the number of most recent block.
        async fn eth_blocknumber(&self) -> Result<U256> {
            self.1.eth_blocknumber().await
        }

        /// Returns the chain ID of the current network
        async fn eth_chainid(&self) -> Result<u64> {
            self.1.eth_chainid().await
        }

        /// Returns information about a block by hash.
        async fn eth_getblockbyhash<H>(&self, hash: H, hydrated: bool) -> Result<Option<Block>>
        where
            H: TryInto<H256> + Send,
            H::Error: Debug + Send,
        {
            self.1.eth_getblockbyhash(hash, hydrated).await
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
            self.1.eth_getblockbynumber(number_or_tag, hydrated).await
        }

        /// Returns transaction count number of block by block hash.
        async fn eth_get_block_transaction_count_by_hash<H>(&self, hash: H) -> Result<u64>
        where
            H: TryInto<H256> + Send,
            H::Error: Debug + Send,
        {
            self.1.eth_get_block_transaction_count_by_hash(hash).await
        }

        /// Returns the number of uncles in a block from a block matching the given block hash
        async fn eth_get_uncle_count_by_block_hash<H>(&self, hash: H) -> Result<u64>
        where
            H: TryInto<H256> + Send,
            H::Error: Debug + Send,
        {
            self.1.eth_get_uncle_count_by_block_hash(hash).await
        }

        /// Returns the number of uncles in a block from a block matching the given block hash
        async fn eth_get_uncle_count_by_block_number<N>(&self, number_or_tag: N) -> Result<u64>
        where
            N: TryInto<BlockNumberOrTag> + Send,
            N::Error: Debug + Send,
        {
            self.1
                .eth_get_uncle_count_by_block_number(number_or_tag)
                .await
        }

        /// Returns an object with data about the sync status or false
        async fn eth_syncing(&mut self) -> Result<SyncingStatus> {
            self.1.eth_syncing().await
        }

        /// Returns the client coinbase address.
        async fn eth_coinbase(&mut self) -> Result<Address> {
            self.1.eth_coinbase().await
        }

        /// Returns a list of addresses owned by client.
        async fn eth_accounts(&mut self) -> Result<Vec<Address>> {
            self.1.eth_accounts().await
        }

        /// Executes a new message call immediately without creating a transaction on the block chain.
        async fn eth_call<TX, BT>(
            &mut self,
            transaction: TX,
            block_number_or_tag: Option<BT>,
        ) -> Result<Bytes>
        where
            TX: TryInto<TypedTransactionRequest> + Send,
            TX::Error: Debug + Send,
            BT: TryInto<BlockNumberOrTag> + Send,
            BT::Error: Debug + Send,
        {
            self.1.eth_call(transaction, block_number_or_tag).await
        }

        /// Generates and returns an estimate of how much gas is necessary to allow the transaction to complete.
        async fn eth_estimate_gas<TX, BT>(
            &mut self,
            transaction: TX,
            block_number_or_tag: Option<BT>,
        ) -> Result<U256>
        where
            TX: TryInto<TypedTransactionRequest> + Send,
            TX::Error: Debug + Send,
            BT: TryInto<BlockNumberOrTag> + Send,
            BT::Error: Debug + Send,
        {
            self.1
                .eth_estimate_gas(transaction, block_number_or_tag)
                .await
        }

        /// Generates an access list for a transaction
        async fn eth_create_accesslist<TX, BT>(
            &mut self,
            transaction: TX,
            block_number_or_tag: Option<BT>,
        ) -> Result<U256>
        where
            TX: TryInto<Transaction> + Send,
            TX::Error: Debug + Send,
            BT: TryInto<BlockNumberOrTag> + Send,
            BT::Error: Debug + Send,
        {
            self.1
                .eth_create_accesslist(transaction, block_number_or_tag)
                .await
        }

        /// Returns the current price gas in wei.
        async fn eth_gas_price(&mut self) -> Result<U256> {
            self.1.eth_gas_price().await
        }

        /// Returns the current maxPriorityFeePerGas per gas in wei.
        async fn eth_max_priority_fee_per_gas(&mut self) -> Result<U256> {
            self.1.eth_max_priority_fee_per_gas().await
        }

        /// Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.
        async fn eth_fee_history<N, BT, RP>(
            &mut self,
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
            self.1
                .eth_fee_history(block_count, newest_block, reward_percentiles)
                .await
        }

        /// Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.
        async fn eth_new_filter<F>(&mut self, filter: F) -> Result<U256>
        where
            F: TryInto<Filter> + Send,
            F::Error: Debug + Send,
        {
            self.1.eth_new_filter(filter).await
        }

        /// Creates new filter in the node,to notify when a new block arrives.
        async fn eth_new_block_filter(&mut self) -> Result<U256> {
            self.1.eth_new_block_filter().await
        }

        /// Creates new filter in the node,to notify when new pending transactions arrive.
        async fn eth_new_pending_transaction_filter(&mut self) -> Result<U256> {
            self.1.eth_new_pending_transaction_filter().await
        }

        /// Uninstalls a filter with given id
        async fn eth_uninstall_filter<N>(&mut self, id: N) -> Result<bool>
        where
            N: TryInto<U256> + Send,
            N::Error: Debug + Send,
        {
            self.1.eth_uninstall_filter(id).await
        }

        /// Polling method for a filter, which returns an arrya of logs which occurred since last poll

        async fn eth_get_filter_changes<N>(&mut self, id: N) -> Result<Option<FilterEvents>>
        where
            N: TryInto<U256> + Send,
            N::Error: Debug + Send,
        {
            self.1.eth_get_filter_changes(id).await
        }

        /// Returns any arrays of all logs matching filter with given id
        async fn eth_get_filter_logs<N>(&mut self, id: N) -> Result<FilterEvents>
        where
            N: TryInto<U256> + Send,
            N::Error: Debug + Send,
        {
            self.1.eth_get_filter_logs(id).await
        }

        /// Returns an array of all logs matching filter with filter description
        async fn eth_get_logs<F>(&mut self, filter: F) -> Result<FilterEvents>
        where
            F: TryInto<Filter> + Send,
            F::Error: Debug + Send,
        {
            self.1.eth_get_logs(filter).await
        }

        /// Returns an RLP encoded transaction signed by the specified account.
        async fn eth_sign_transaction<T>(&mut self, transaction: T) -> Result<Bytes>
        where
            T: TryInto<Transaction> + Send,
            T::Error: Debug + Send,
        {
            self.1.eth_sign_transaction(transaction).await
        }

        /// Returns the balance of the account given address.
        async fn eth_get_balance<A>(&mut self, address: A) -> Result<U256>
        where
            A: TryInto<Address> + Send,
            A::Error: Debug + Send,
        {
            self.1.eth_get_balance(address).await
        }

        /// Returns the number of transactions sent from an address
        async fn eth_get_transaction_count<A>(&mut self, address: A) -> Result<U256>
        where
            A: TryInto<Address> + Send,
            A::Error: Debug + Send,
        {
            self.1.eth_get_transaction_count(address).await
        }

        /// Submit a raw transaction.
        async fn eth_send_raw_transaction<B>(&mut self, raw: B) -> Result<H256>
        where
            B: TryInto<Bytes> + Send,
            B::Error: Debug + Send,
        {
            self.1.eth_send_raw_transaction(raw).await
        }

        async fn eth_get_transaction_by_hash<H>(
            &mut self,
            tx_hash: H,
        ) -> Result<Option<Transaction>>
        where
            H: TryInto<H256> + Send,
            H::Error: Debug + Send,
        {
            self.1.eth_get_transaction_by_hash(tx_hash).await
        }

        /// Returns the receipt of a transaction by transaction hash
        async fn eth_get_transaction_receipt<H>(
            &mut self,
            tx_hash: H,
        ) -> Result<Option<TransactionReceipt>>
        where
            H: TryInto<H256> + Send,
            H::Error: Debug + Send,
        {
            self.1.eth_get_transaction_receipt(tx_hash).await
        }
    }
}

#[cfg(feature = "clients")]
pub use with_client::*;
