//! [`quic`](https://www.wikiwand.com/en/QUIC) protocol implementation for the [**RASI**](rasi) runtime,
//! is an asynchronous runtime wrapper for [`quiche`](https://docs.rs/quiche/latest/quiche/).
//!
//! You can use any `RASI`-compatible asynchronous runtime to drive this implementation,
//! such as [`rasi-default`](https://docs.rs/rasi-default/latest/rasi_default/), etc,.
//!
//! # Example

mod config;
mod conn;
mod errors;
mod listener;

pub use config::*;
pub use conn::*;
pub use listener::*;

#[cfg(test)]
mod tests {

    use futures::TryStreamExt;
    use parking_lot::Once;
    use quiche::RecvInfo;

    use rasi_default::time::register_mio_timer;

    use super::*;

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
            register_mio_timer();
        })
    }

    #[futures_test::test]
    async fn test_connect() {
        init();

        let laddr = "127.0.0.1:1200".parse().unwrap();
        let raddr = "127.0.0.1:1201".parse().unwrap();

        let client = QuicConnState::connect(None, laddr, raddr, &mut mock_config(false)).unwrap();

        assert!(!client.is_established().await);

        let config = mock_config(true);

        let max_send_udp_payload_size = config.max_send_udp_payload_size;

        let (listener, _incoming, mut sender) = QuicListenerState::new(config).unwrap();

        let mut buf = vec![0; max_send_udp_payload_size];

        loop {
            let (send_size, send_info) = client.send(&mut buf).await.unwrap();

            let (_, buf) = listener
                .recv(
                    &mut buf[..send_size],
                    RecvInfo {
                        from: send_info.from,
                        to: send_info.to,
                    },
                )
                .await
                .unwrap();

            if let Some(mut buf) = buf {
                client
                    .recv(
                        &mut buf,
                        RecvInfo {
                            from: send_info.to,
                            to: send_info.from,
                        },
                    )
                    .await
                    .unwrap();
            } else {
                if let Some((mut buf, send_info)) = sender.try_next().await.unwrap() {
                    client
                        .recv(
                            &mut buf,
                            RecvInfo {
                                from: send_info.from,
                                to: send_info.to,
                            },
                        )
                        .await
                        .unwrap();
                }
            }

            if client.is_established().await {
                break;
            }
        }
    }
}
