pub mod contracts;

#[cfg(test)]
mod tests {

    use super::contracts::personal_wallet::PersonalWallet;

    use std::sync::{Once, OnceLock};

    use futures_jsonrpcv2::{client::JsonRpcClient, rasi::http::HttpJsonRpcClient};
    use rasi_mio::{net::register_mio_network, timer::register_mio_timer};
    use reweb3::{clients::JsonRpcProvider, runtimes::Address};

    fn init() -> JsonRpcProvider {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            // pretty_env_logger::init_timed();
            register_mio_network();
            register_mio_timer();
        });

        static CLIENT: OnceLock<JsonRpcClient> = OnceLock::new();

        CLIENT
            .get_or_init(|| {
                HttpJsonRpcClient::new(
                    "https://mainnet.infura.io/v3/efdbc2d092c34ec4a161eeb7991ea6cc",
                )
                .set_use_server_name_indication(false)
                .create()
                .unwrap()
            })
            .clone()
            .into()
    }

    #[test]
    fn wallet_new() {
        let provider = init();
        let _wallet = PersonalWallet::new(provider, Address::zero_address());
    }
}
