use std::sync::Once;

use futures::{AsyncReadExt, AsyncWriteExt, TryStreamExt};
use futures_quic::{QuicConn, QuicConnect, QuicListener, QuicListenerBind};
use quiche::Config;
use rasi::task::spawn_ok;
use rasi_mio::{net::register_mio_network, timer::register_mio_timer};

fn init() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        // pretty_env_logger::init();
        register_mio_network();
        register_mio_timer();
    })
}

fn mock_config(is_server: bool) -> Config {
    use std::path::Path;

    let mut config = Config::new(quiche::PROTOCOL_VERSION).unwrap();

    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1024 * 1024);
    config.set_initial_max_stream_data_bidi_remote(1024 * 1024);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);

    config.verify_peer(true);

    // if is_server {
    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    log::debug!("test run dir {:?}", root_path);

    if is_server {
        config
            .load_cert_chain_from_pem_file(
                root_path.join("../../cert/server.crt").to_str().unwrap(),
            )
            .unwrap();

        config
            .load_priv_key_from_pem_file(root_path.join("../../cert/server.key").to_str().unwrap())
            .unwrap();
    } else {
        config
            .load_cert_chain_from_pem_file(
                root_path.join("../../cert/client.crt").to_str().unwrap(),
            )
            .unwrap();

        config
            .load_priv_key_from_pem_file(root_path.join("../../cert/client.key").to_str().unwrap())
            .unwrap();
    }

    config
        .load_verify_locations_from_file(root_path.join("../../cert/rasi_ca.pem").to_str().unwrap())
        .unwrap();

    config.set_application_protos(&[b"test"]).unwrap();

    config.set_max_idle_timeout(50000);

    config.set_initial_max_data(10_000_000);
    config.set_disable_active_migration(false);

    config
}

#[futures_test::test]
async fn test_echo() {
    init();

    let listener = QuicListener::bind("127.0.0.1:0", mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn_ok(async move {
        while let Some(conn) = listener.incoming().try_next().await.unwrap() {
            let mut incoming = conn.into_incoming();
            while let Some(mut stream) = incoming.try_next().await.unwrap() {
                loop {
                    let mut buf = vec![0; 100];

                    let read_size = stream.read(&mut buf).await.unwrap();

                    if read_size == 0 {
                        break;
                    }

                    stream.write_all(&buf[..read_size]).await.unwrap();
                }
            }
        }
    });

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    let mut stream = client.open(false).await.unwrap();

    for _ in 0..101 {
        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
    }
}

#[futures_test::test]
async fn test_echo_per_stream() {
    init();
    // pretty_env_logger::init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(1);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn_ok(async move {
        while let Some(conn) = listener.incoming().try_next().await.unwrap() {
            let mut incoming = conn.into_incoming();

            while let Some(mut stream) = incoming.try_next().await.unwrap() {
                spawn_ok(async move {
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

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    for _ in 0..101 {
        let mut stream = client.open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
    }
}

#[futures_test::test]
async fn test_connect_server_close() {
    init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn_ok(async move {
        while let Some(conn) = listener.incoming().try_next().await.unwrap() {
            let mut incoming = conn.into_incoming();
            while let Some(mut stream) = incoming.try_next().await.unwrap() {
                let mut buf = vec![0; 100];
                _ = stream.read(&mut buf).await.unwrap();

                break;
            }
        }
    });

    for _ in 0..101 {
        let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
            .await
            .unwrap();

        let mut stream = client.open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        stream
            .read(&mut buf)
            .await
            .expect_err("Server close the connection");
    }
}

#[futures_test::test]
async fn test_connect_client_close() {
    init();
    // pretty_env_logger::init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn_ok(async move {
        while let Some(conn) = listener.incoming().try_next().await.unwrap() {
            spawn_ok(async move {
                let mut incoming = conn.into_incoming();

                while let Some(mut stream) = incoming.try_next().await.unwrap() {
                    spawn_ok(async move {
                        loop {
                            let mut buf = vec![0; 100];
                            let read_size = stream.read(&mut buf).await.unwrap();

                            stream.write_all(&buf[..read_size]).await.unwrap();

                            stream
                                .read(&mut buf)
                                .await
                                .expect_err("Client close the connection");

                            break;
                        }
                    });
                }
            })
        }
    });

    for _ in 0..101 {
        let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
            .await
            .unwrap();

        let mut stream = client.open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
    }
}

#[futures_test::test]
async fn test_stream_server_close() {
    init();
    // pretty_env_logger::init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn_ok(async move {
        while let Some(conn) = listener.incoming().try_next().await.unwrap() {
            let mut incoming = conn.into_incoming();

            while let Some(mut stream) = incoming.try_next().await.unwrap() {
                let mut buf = vec![0; 100];
                let read_size = stream.read(&mut buf).await.unwrap();

                stream.write_all(&buf[..read_size]).await.unwrap();
            }
        }
    });

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    for _ in 0..101 {
        let mut stream = client.open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
    }
}

#[futures_test::test]
async fn test_stream_server_close_with_fin() {
    init();
    // pretty_env_logger::init_timed();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn_ok(async move {
        while let Some(conn) = listener.incoming().try_next().await.unwrap() {
            let mut incoming = conn.into_incoming();
            while let Some(stream) = incoming.try_next().await.unwrap() {
                log::trace!("server accept stream, id={}", stream.id());

                loop {
                    let mut buf = vec![0; 100];
                    let (read_size, fin) = stream.recv(&mut buf).await.unwrap();

                    log::trace!("server recv, fin={}", fin);

                    if fin {
                        break;
                    }

                    stream.send(&buf[..read_size], true).await.unwrap();

                    log::trace!("server send",);
                }
            }
        }
    });

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    for i in 0..10000 {
        let stream = client.open(false).await.unwrap();

        stream.send(b"hello world", false).await.unwrap();

        log::trace!("client send {}", i);

        let mut buf = vec![0; 100];

        let (read_size, fin) = stream.recv(&mut buf).await.unwrap();

        log::trace!("client recv {}", i);

        assert_eq!(&buf[..read_size], b"hello world");
        assert!(fin);
    }
}
