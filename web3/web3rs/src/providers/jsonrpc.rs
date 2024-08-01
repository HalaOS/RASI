//! A Provider is an abstraction of a connection to the Ethereum network,
//! providing a concise, consistent interface to standard Ethereum node
//! functionality.

use crate::{
    errors::Error,
    primitives::{h256::H256, hex::HexError, u256::U256},
};

use futures_jsonrpcv2::client::JsonRpcClient;

use super::Block;

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
    pub async fn eth_blocknumber(&self) -> Result<U256, Error> {
        let value: String = self
            .0
            .clone()
            .call("eth_blockNumber", Vec::<String>::new())
            .await?;

        Ok(U256::from_str_hex(&value)?)
    }

    /// Returns the chain ID of the current network
    pub async fn eth_chainid(&self) -> Result<u64, Error> {
        let value: String = self.0.clone().call("eth_chainId", ()).await?;
        let value = U256::from_str_hex(&value)?;

        Ok(value.as_u64())
    }

    pub async fn eth_getblockbyhash<H: TryInto<H256, Error = HexError>>(
        &self,
        hash: H,
        hydrated: bool,
    ) -> Result<Option<Block>, Error> {
        let hash: H256 = hash.try_into()?;

        Ok(self
            .0
            .clone()
            .call("eth_getBlockByHash", (hash.to_string(), hydrated))
            .await?)
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
            pretty_env_logger::init_timed();
            register_mio_network();
            register_mio_timer();
        });

        HttpJsonRpcClient::new("https://eth-mainnet.token.im")
            .set_use_server_name_indication(true)
            .create()
            .unwrap()
            .into()
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
}
