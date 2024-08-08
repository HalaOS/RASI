use std::fmt::Debug;

use async_trait::async_trait;
use serde::Serialize;

use crate::{
    eip::{eip2718::TypedTransactionRequest, eip712::TypedData},
    errors::Result,
    prelude::Address,
    primitives::{Bytes, Eip1559Signature},
};

/// Represent a data signer for etherenum client.
#[async_trait]
pub trait Signer {
    /// Signs the transaction request and returns the signed transaction request in [`rlp`](crate::rlp) format.
    async fn sign_transaction<T>(
        &self,
        request: T,
        with_account: Option<&Address>,
    ) -> Result<Bytes>
    where
        T: TryInto<TypedTransactionRequest> + Send,
        T::Error: Debug + Send;

    /// Calculate the signature of the [`TypedData`] defined in [`eip-712`](https://eips.ethereum.org/EIPS/eip-712)
    async fn sign_typed_data<V>(
        &self,
        typed_data: TypedData<V>,
        with_account: Option<&Address>,
    ) -> Result<Eip1559Signature>
    where
        V: Serialize + Send;

    /// Returns the signer's account addresss.
    async fn signer_accounts(&self) -> Result<Vec<Address>>;
}

#[cfg(feature = "clients")]
#[cfg_attr(docsrs, doc(cfg(feature = "clients")))]
mod with_client {

    use reweb3_num::types::U256;

    use crate::{
        clients::{
            Block, BlockNumberOrTag, Client, ClientExt, FeeHistory, Filter, FilterEvents,
            SyncingStatus, Transaction, TransactionReceipt,
        },
        eip::eip2718::LegacyTransactionRequest,
        errors::Error,
        prelude::keccak256,
        primitives::{balance::TransferOptions, Address, Bytes, H256},
    };

    use super::*;

    /// A combinated trait of [`Signer`] and [`Client`]
    #[async_trait]
    pub trait SignerWithProvider: Signer + ClientExt + Send + Sync {
        /// Deploy a contract with `bytecode`.
        async fn deploy_contract(
            &self,
            bytecode: &str,
            signature: &str,
            mut call_data: Bytes,
            ops: TransferOptions,
        ) -> Result<H256> {
            let mut bytecode: Bytes = bytecode.parse()?;

            bytecode.0.append(&mut call_data.0);

            self.send_raw_transaction(signature, None, call_data, ops)
                .await
        }

        /// Call contract nonpayable/payable function.
        async fn call_contract_with_transaction(
            &self,
            signature: &str,
            contract_address: &Address,
            call_data: Bytes,
            ops: TransferOptions,
        ) -> Result<H256> {
            self.send_raw_transaction(signature, Some(&contract_address), call_data, ops)
                .await
        }

        /// Sign and send a raw transaction.
        async fn send_raw_transaction(
            &self,
            method_name: &str,
            to: Option<&Address>,
            mut call_data: Bytes,
            ops: TransferOptions,
        ) -> Result<H256> {
            let address = {
                let mut accounts = self.signer_accounts().await?;

                if accounts.is_empty() {
                    return Err(Error::SignerAccounts);
                }

                accounts.remove(0)
            };

            // Get nonce first.
            let nonce = self.eth_get_transaction_count(address.clone()).await?;

            log::debug!(
                target: method_name,
                "Fetch account {} nonce, {}",
                address.to_checksum_string(),
                nonce
            );

            // Get chain id
            let chain_id = self.eth_chainid().await?;

            log::debug!(target: method_name, "Fetch chain_id, {}", chain_id);

            let call_data = if to.is_some() {
                let mut selector_name = keccak256(method_name.as_bytes()).0[0..4].to_vec();

                selector_name.append(&mut call_data.0);

                selector_name.into()
            } else {
                call_data
            };

            let mut tx = LegacyTransactionRequest {
                chain_id: Some(chain_id.into()),
                nonce: Some(nonce),
                to: to.map(|c| c.clone()),
                data: Some(call_data),
                value: ops.value,
                ..Default::default()
            };

            // estimate gas
            let gas = self
                .eth_estimate_gas(tx.clone(), None::<BlockNumberOrTag>)
                .await?;

            log::debug!(target: method_name, "Fetch estimate gas, {}", gas);

            tx.gas = Some(gas);

            // Get gas price

            let gas_price = if let Some(gas_price) = ops.gas_price {
                gas_price
            } else {
                self.eth_gas_price().await?
            };

            log::debug!(target: method_name, "Fetch gas price, {}", gas_price);

            tx.gas_price = Some(gas_price);

            log::debug!(
                target: method_name,
                "Try sign transaction, {}",
                serde_json::to_string(&tx)?,
            );

            let signed_tx = self.sign_transaction(tx, Some(&address)).await?;

            log::debug!(
                target: method_name,
                "Signed transaction, {}",
                signed_tx.to_string()
            );

            let hash = self.eth_send_raw_transaction(signed_tx).await?;

            log::debug!(target: method_name, "Send transaction success, {}", hash);

            Ok(hash)
        }

        /// Get the balance of this signer's first account.
        async fn balance(&self) -> Result<U256> {
            let addresses = self.signer_accounts().await?;

            if addresses.is_empty() {
                return Err(Error::Other("Signer account list is empty".to_string()));
            }

            Ok(self.eth_get_balance(addresses[0].clone()).await?)
        }
    }

    #[async_trait]
    impl<S, C> Signer for (S, C)
    where
        S: Signer + Sync + Send + Unpin,
        C: Client + Sync + Send + Unpin,
    {
        /// Signs the transaction request and returns the signed transaction request in [`rlp`](crate::rlp) format.
        async fn sign_transaction<T>(
            &self,
            request: T,
            with_account: Option<&Address>,
        ) -> Result<Bytes>
        where
            T: TryInto<TypedTransactionRequest> + Send,
            T::Error: Debug + Send,
        {
            self.0.sign_transaction(request, with_account).await
        }

        /// Calculate the signature of the [`TypedData`] defined in [`eip-712`](https://eips.ethereum.org/EIPS/eip-712)
        async fn sign_typed_data<V>(
            &self,
            typed_data: TypedData<V>,
            with_account: Option<&Address>,
        ) -> Result<Eip1559Signature>
        where
            V: Serialize + Send,
        {
            self.0.sign_typed_data(typed_data, with_account).await
        }

        /// Returns the signer's account addresss.
        async fn signer_accounts(&self) -> Result<Vec<Address>> {
            self.0.signer_accounts().await
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
        async fn eth_syncing(&self) -> Result<SyncingStatus> {
            self.1.eth_syncing().await
        }

        /// Returns the client coinbase address.
        async fn eth_coinbase(&self) -> Result<Address> {
            self.1.eth_coinbase().await
        }

        /// Returns a list of addresses owned by client.
        async fn eth_accounts(&self) -> Result<Vec<Address>> {
            self.1.eth_accounts().await
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
            self.1.eth_call(transaction, block_number_or_tag).await
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
            self.1
                .eth_estimate_gas(transaction, block_number_or_tag)
                .await
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
            self.1
                .eth_create_accesslist(transaction, block_number_or_tag)
                .await
        }

        /// Returns the current price gas in wei.
        async fn eth_gas_price(&self) -> Result<U256> {
            self.1.eth_gas_price().await
        }

        /// Returns the current maxPriorityFeePerGas per gas in wei.
        async fn eth_max_priority_fee_per_gas(&self) -> Result<U256> {
            self.1.eth_max_priority_fee_per_gas().await
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
            self.1
                .eth_fee_history(block_count, newest_block, reward_percentiles)
                .await
        }

        /// Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.
        async fn eth_new_filter<F>(&self, filter: F) -> Result<U256>
        where
            F: TryInto<Filter> + Send,
            F::Error: Debug + Send,
        {
            self.1.eth_new_filter(filter).await
        }

        /// Creates new filter in the node,to notify when a new block arrives.
        async fn eth_new_block_filter(&self) -> Result<U256> {
            self.1.eth_new_block_filter().await
        }

        /// Creates new filter in the node,to notify when new pending transactions arrive.
        async fn eth_new_pending_transaction_filter(&self) -> Result<U256> {
            self.1.eth_new_pending_transaction_filter().await
        }

        /// Uninstalls a filter with given id
        async fn eth_uninstall_filter<N>(&self, id: N) -> Result<bool>
        where
            N: TryInto<U256> + Send,
            N::Error: Debug + Send,
        {
            self.1.eth_uninstall_filter(id).await
        }

        /// Polling method for a filter, which returns an arrya of logs which occurred since last poll

        async fn eth_get_filter_changes<N>(&self, id: N) -> Result<Option<FilterEvents>>
        where
            N: TryInto<U256> + Send,
            N::Error: Debug + Send,
        {
            self.1.eth_get_filter_changes(id).await
        }

        /// Returns any arrays of all logs matching filter with given id
        async fn eth_get_filter_logs<N>(&self, id: N) -> Result<FilterEvents>
        where
            N: TryInto<U256> + Send,
            N::Error: Debug + Send,
        {
            self.1.eth_get_filter_logs(id).await
        }

        /// Returns an array of all logs matching filter with filter description
        async fn eth_get_logs<F>(&self, filter: F) -> Result<FilterEvents>
        where
            F: TryInto<Filter> + Send,
            F::Error: Debug + Send,
        {
            self.1.eth_get_logs(filter).await
        }

        /// Returns an RLP encoded transaction signed by the specified account.
        async fn eth_sign_transaction<T>(&self, transaction: T) -> Result<Bytes>
        where
            T: TryInto<Transaction> + Send,
            T::Error: Debug + Send,
        {
            self.1.eth_sign_transaction(transaction).await
        }

        /// Returns the balance of the account given address.
        async fn eth_get_balance<A>(&self, address: A) -> Result<U256>
        where
            A: TryInto<Address> + Send,
            A::Error: Debug + Send,
        {
            self.1.eth_get_balance(address).await
        }

        /// Returns the number of transactions sent from an address
        async fn eth_get_transaction_count<A>(&self, address: A) -> Result<U256>
        where
            A: TryInto<Address> + Send,
            A::Error: Debug + Send,
        {
            self.1.eth_get_transaction_count(address).await
        }

        /// Submit a raw transaction.
        async fn eth_send_raw_transaction<B>(&self, raw: B) -> Result<H256>
        where
            B: TryInto<Bytes> + Send,
            B::Error: Debug + Send,
        {
            self.1.eth_send_raw_transaction(raw).await
        }

        async fn eth_get_transaction_by_hash<H>(&self, tx_hash: H) -> Result<Option<Transaction>>
        where
            H: TryInto<H256> + Send,
            H::Error: Debug + Send,
        {
            self.1.eth_get_transaction_by_hash(tx_hash).await
        }

        /// Returns the receipt of a transaction by transaction hash
        async fn eth_get_transaction_receipt<H>(
            &self,
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
