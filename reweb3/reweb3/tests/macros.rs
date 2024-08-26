use std::sync::{Once, OnceLock};

use futures_jsonrpcv2::rasi::http::HttpJsonRpcClient;
use rasi_mio::{net::register_mio_network, timer::register_mio_timer};
use reweb3::{clients::JsonRpcProvider, hardhat_artifact, prelude::Address};

hardhat_artifact!("tests/abi.json");

fn init() -> JsonRpcProvider {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        // pretty_env_logger::init_timed();
        register_mio_network();
        register_mio_timer();
    });

    static CLIENT: OnceLock<JsonRpcProvider> = OnceLock::new();

    CLIENT
        .get_or_init(|| {
            HttpJsonRpcClient::new("https://mainnet.infura.io/v3/efdbc2d092c34ec4a161eeb7991ea6cc")
                .set_use_server_name_indication(false)
                .create()
                .unwrap()
                .into()
        })
        .clone()
}

#[futures_test::test]
async fn test_contract() {
    let provider = init();

    let _wallet = personal_wallet::PersonalWallet::new(provider, Address::zero_address());
}
