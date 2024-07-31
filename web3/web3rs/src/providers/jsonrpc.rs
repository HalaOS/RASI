//! A Provider is an abstraction of a connection to the Ethereum network,
//! providing a concise, consistent interface to standard Ethereum node
//! functionality.

use crate::{errors::Error, primitives::u256::U256};

use futures_jsonrpcv2::client::JsonRpcClient;

/// The JSON-RPC API is a popular method for interacting with Ethereum and is
/// available in all major Ethereum node implementations (e.g. Geth and Parity)
/// as well as many third-party web services (e.g. INFURA)
pub struct JsonRpcProvider(
    /// jsonrpc2.0 client for this provider.
    JsonRpcClient,
);

impl From<JsonRpcClient> for JsonRpcProvider {
    fn from(value: JsonRpcClient) -> Self {
        Self(value)
    }
}

impl JsonRpcProvider {
    /// Returns the number of most recent block.
    pub async fn eth_block_number(&self) -> Result<U256, Error> {
        let value: String = self
            .0
            .clone()
            .call("eth_blockNumber", Vec::<String>::new())
            .await?;

        Ok(U256::from_str_hex(&value)?)
    }

    /// Returns the chain ID of the current network
    pub async fn eth_chain_id(&self) -> Result<u64, Error> {
        let value: String = self.0.clone().call("eth_chainId", ()).await?;
        let value = U256::from_str_hex(&value)?;

        Ok(value.as_u64())
    }
}

#[cfg(all(test, feature = "test-providers"))]
mod tests {
    use std::sync::Once;

    use futures_jsonrpcv2::rasi::http::HttpJsonRpcClient;
    use rasi_mio::{net::register_mio_network, timer::register_mio_timer};

    use super::*;

    fn init() -> JsonRpcProvider {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            // pretty_env_logger::init_timed();
            register_mio_network();
            register_mio_timer();
        });

        HttpJsonRpcClient::new("HTTP://127.0.0.1:1545")
            .create()
            .unwrap()
            .into()
    }

    #[futures_test::test]
    async fn test_eth_block_number() {
        let provider = init();

        provider.eth_block_number().await.unwrap();
    }

    #[futures_test::test]
    async fn test_eth_chain_id() {
        let provider = init();

        provider.eth_chain_id().await.unwrap();
    }
}
