use futures::{AsyncReadExt, AsyncWriteExt};
use rasi::executor::spawn;
use rasi_ext::net::quic::{Config, QuicConn, QuicListener};

mod init;

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

    config.set_initial_max_data(10_000_000);
    config.set_disable_active_migration(false);

    config
}

#[futures_test::test]
async fn test_echo() {
    init::init();
    // pretty_env_logger::init();

    let listener = QuicListener::bind("127.0.0.1:0", mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    spawn(async move {
        while let Some(conn) = listener.accept().await {
            while let Some(mut stream) = conn.stream_accept().await {
                loop {
                    let mut buf = vec![0; 100];
                    let read_size = stream.read(&mut buf).await.unwrap();

                    stream.write_all(&buf[..read_size]).await.unwrap();
                }
            }
        }
    });

    let mut stream = client.stream_open(false).await.unwrap();

    for _ in 0..100 {
        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
    }
}

#[futures_test::test]
async fn test_echo_per_stream() {
    init::init();
    // pretty_env_logger::init();

    let listener = QuicListener::bind("127.0.0.1:0", mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    spawn(async move {
        while let Some(conn) = listener.accept().await {
            while let Some(mut stream) = conn.stream_accept().await {
                spawn(async move {
                    loop {
                        let mut buf = vec![0; 100];
                        let read_size = stream.read(&mut buf).await.unwrap();

                        if read_size == 0 {
                            break;
                        }

                        stream.write_all(&buf[..read_size]).await.unwrap();
                    }
                })
            }
        }
    });

    for _ in 0..1000 {
        let mut stream = client.stream_open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");

        while client.peer_streams_left_bidi().await == 0 {
            println!("===========");
        }
    }
}
