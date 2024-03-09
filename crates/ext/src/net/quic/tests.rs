use parking_lot::Once;
use rasi_default::{
    executor::register_futures_executor, net::register_mio_network, time::register_mio_timer,
};

use super::{Config, QuicListener};

fn mock_config(is_server: bool) -> Config {
    use std::path::Path;

    let mut config = Config::new();

    config.verify_peer(true);

    // if is_server {
    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    log::debug!("test run dir {:?}", root_path);

    if is_server {
        config
            .load_cert_chain_from_pem_file(root_path.join("cert/server.crt").to_str().unwrap())
            .unwrap();

        config
            .load_priv_key_from_pem_file(root_path.join("cert/server.key").to_str().unwrap())
            .unwrap();
    } else {
        config
            .load_cert_chain_from_pem_file(root_path.join("cert/client.crt").to_str().unwrap())
            .unwrap();

        config
            .load_priv_key_from_pem_file(root_path.join("cert/client.key").to_str().unwrap())
            .unwrap();
    }

    config
        .load_verify_locations_from_file(root_path.join("cert/rasi_ca.pem").to_str().unwrap())
        .unwrap();

    config
        .set_application_protos(&[b"hq-interop", b"hq-29", b"hq-28", b"hq-27", b"http/0.9"])
        .unwrap();

    config.set_max_idle_timeout(5000);
    config.set_initial_max_data(10_000_000);
    config.set_disable_active_migration(false);

    config
}

fn init() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        register_mio_network();
        register_mio_timer();
        register_futures_executor(10).unwrap();
    })
}

#[futures_test::test]
async fn test_echo() {
    init();
    let listener = QuicListener::bind("127.0.0.1", mock_config(true))
        .await
        .unwrap();
}
