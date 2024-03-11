use std::{io, time::Duration};

use futures::{AsyncReadExt, AsyncWriteExt};
use rasi::{executor::spawn, time::sleep};
use rasi_ext::net::quic::{Config, QuicConn, QuicConnPool, QuicListener};

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

    config.set_max_idle_timeout(50000);

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

    spawn(async move {
        while let Some(conn) = listener.accept().await {
            while let Some(mut stream) = conn.stream_accept().await {
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
    pretty_env_logger::init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

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

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    for _ in 0..10000 {
        let mut stream = client.stream_open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
    }
}

#[futures_test::test]
async fn test_connect_server_close() {
    init::init();
    // pretty_env_logger::init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn(async move {
        while let Some(conn) = listener.accept().await {
            while let Some(mut stream) = conn.stream_accept().await {
                let mut buf = vec![0; 100];
                _ = stream.read(&mut buf).await.unwrap();

                break;
            }
        }
    });

    for _ in 0..100 {
        let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
            .await
            .unwrap();

        let mut stream = client.stream_open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        assert_eq!(stream.read(&mut buf).await.unwrap(), 0);
    }
}

#[futures_test::test]
async fn test_connect_client_close() {
    init::init();
    // pretty_env_logger::init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

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
                });
            }
        }
    });

    for _ in 0..100 {
        let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
            .await
            .unwrap();

        let mut stream = client.stream_open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
    }
}

#[futures_test::test]
async fn test_stream_server_close() {
    init::init();
    // pretty_env_logger::init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn(async move {
        while let Some(conn) = listener.accept().await {
            while let Some(mut stream) = conn.stream_accept().await {
                let mut buf = vec![0; 100];
                let read_size = stream.read(&mut buf).await.unwrap();

                stream.write_all(&buf[..read_size]).await.unwrap();
            }
        }
    });

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    for _ in 0..100 {
        let mut stream = client.stream_open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
    }
}

#[futures_test::test]
async fn test_stream_server_close_with_fin() {
    init::init();
    // pretty_env_logger::init();

    let laddrs = ["127.0.0.1:0".parse().unwrap()].repeat(10);

    let listener = QuicListener::bind(laddrs.as_slice(), mock_config(true))
        .await
        .unwrap();

    let raddr = listener.local_addrs().collect::<Vec<_>>()[0].clone();

    spawn(async move {
        while let Some(conn) = listener.accept().await {
            while let Some(mut stream) = conn.stream_accept().await {
                loop {
                    let mut buf = vec![0; 100];
                    let read_size = stream.read(&mut buf).await.unwrap();

                    if read_size == 0 {
                        break;
                    }

                    stream.stream_send(&buf[..read_size], true).await.unwrap();
                }
            }
        }
    });

    let client = QuicConn::connect(None, "127.0.0.1:0", raddr, &mut mock_config(false))
        .await
        .unwrap();

    for _ in 0..100 {
        let mut stream = client.stream_open(false).await.unwrap();

        stream.write_all(b"hello world").await.unwrap();

        let mut buf = vec![0; 100];

        let (read_size, fin) = stream.stream_recv(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], b"hello world");
        assert!(fin);
    }
}

#[futures_test::test]
async fn test_conn_pool() {
    init::init();
    let mut config = mock_config(true);

    config.set_initial_max_streams_bidi(2);

    let listener = QuicListener::bind("127.0.0.1:0", config).await.unwrap();

    let raddr = listener.local_addrs().next().unwrap().clone();

    spawn(async move {
        let conn = listener.accept().await.unwrap();

        while let Some(mut stream) = conn.stream_accept().await {
            spawn(async move {
                let mut buf = vec![0; 1024];

                let read_size = stream.read(&mut buf).await.unwrap();

                stream
                    .stream_send(&mut buf[..read_size], true)
                    .await
                    .unwrap();
            });
        }
    });

    let mut conn_pool = QuicConnPool::new(None, raddr, mock_config(false)).unwrap();

    conn_pool.set_max_conns(10);

    let mut streams = vec![];

    for _ in 0..10 {
        let stream = conn_pool.stream_open().await.unwrap();

        // stream.write(b"hello").await.unwrap();

        streams.push(stream);
    }

    let err = conn_pool.stream_open().await.expect_err("WouldBlock");

    assert_eq!(err.kind(), io::ErrorKind::WouldBlock);

    streams.drain(..);

    loop {
        // waiting stream closed.
        sleep(Duration::from_secs(1)).await;

        if conn_pool.stream_open().await.is_ok() {
            break;
        }
    }
}

#[futures_test::test]
async fn test_conn_pool_reconnect() {
    init::init();
    // pretty_env_logger::init_timed();

    let mut config = mock_config(true);

    config.set_initial_max_streams_bidi(2);

    let listener = QuicListener::bind("127.0.0.1:0", config).await.unwrap();

    let raddr = listener.local_addrs().next().unwrap().clone();

    spawn(async move {
        while let Some(conn) = listener.accept().await {
            drop(conn);
        }
    });

    let mut conn_pool = QuicConnPool::new(None, raddr, mock_config(false)).unwrap();

    conn_pool.set_max_conns(1);

    for i in 0..10 {
        log::trace!("open stream {}", i);

        let mut stream = conn_pool.stream_open().await.expect("Reconnect");

        stream.write(b"hello").await.unwrap();

        loop {
            if let Err(err) = stream.write(b"hello").await {
                assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
                break;
            }

            sleep(Duration::from_millis(10)).await;
        }
    }
}
