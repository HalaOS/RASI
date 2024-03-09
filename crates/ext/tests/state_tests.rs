use futures::{io, StreamExt, TryStreamExt};

use quiche::RecvInfo;

use rasi_ext::net::quic::{
    Config, QuicConnState, QuicListenerState, QuicServerStateIncoming, QuicServerStateSender,
};

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

    config.set_max_idle_timeout(5000);
    config.set_initial_max_data(10_000_000);
    config.set_disable_active_migration(false);

    config
}

struct MockQuic {
    client: QuicConnState,
    listener: QuicListenerState,
    listener_sender: QuicServerStateSender,
    listener_incoming: QuicServerStateIncoming,
}

impl MockQuic {
    async fn new() -> io::Result<Self> {
        let laddr = "127.0.0.1:1200".parse().unwrap();
        let raddr = "127.0.0.1:1201".parse().unwrap();

        let client = QuicConnState::connect(None, laddr, raddr, &mut mock_config(false)).unwrap();

        assert!(!client.is_established().await);

        let config = mock_config(true);

        let max_send_udp_payload_size = config.max_send_udp_payload_size;

        let (listener, listener_incoming, mut listener_sender) =
            QuicListenerState::new(config).unwrap();

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
                if let Some((mut buf, send_info)) = listener_sender.try_next().await.unwrap() {
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
                return Ok(Self {
                    client,
                    listener,
                    listener_incoming,
                    listener_sender,
                });
            }
        }
    }

    async fn send_to_server(&mut self) -> io::Result<()> {
        let mut buf = vec![0; 65535];

        let (read_size, send_info) = self.client.send(&mut buf).await?;

        let (_, buf) = self
            .listener
            .recv(
                &mut buf[..read_size],
                RecvInfo {
                    from: send_info.from,
                    to: send_info.to,
                },
            )
            .await?;

        assert!(buf.is_none());

        Ok(())
    }

    async fn send_to_client(&mut self) -> io::Result<()> {
        if let Some((mut buf, send_info)) = self.listener_sender.try_next().await? {
            self.client
                .recv(
                    &mut buf,
                    RecvInfo {
                        from: send_info.from,
                        to: send_info.to,
                    },
                )
                .await?;
        }

        Ok(())
    }
}

#[futures_test::test]
async fn test_connect() {
    init::init();

    let mut mock = MockQuic::new().await.unwrap();

    let stream_id = mock.client.stream_open(true).await.unwrap();

    mock.send_to_server().await.unwrap();

    // connection is established now.
    mock.send_to_client().await.unwrap();

    mock.client
        .stream_send(stream_id, b"", false)
        .await
        .unwrap();

    // trigger newly incoming connection event.
    mock.send_to_server().await.unwrap();

    assert!(mock.listener_incoming.next().await.is_some());
}

#[futures_test::test]
async fn test_connect_trigger_fin() {
    init::init();

    let mut mock = MockQuic::new().await.unwrap();

    let stream_id = mock.client.stream_open(true).await.unwrap();

    mock.send_to_server().await.unwrap();

    // connection is established now.
    mock.send_to_client().await.unwrap();

    mock.client.stream_send(stream_id, b"", true).await.unwrap();

    // trigger newly incoming connection event.
    mock.send_to_server().await.unwrap();

    assert!(mock.listener_incoming.next().await.is_some());
}

#[futures_test::test]
async fn test_stream() {
    init::init();

    let mut mock = MockQuic::new().await.unwrap();

    let stream_id = mock.client.stream_open(true).await.unwrap();

    assert_eq!(stream_id, 4);

    let stream_id = mock.client.stream_open(true).await.unwrap();

    assert_eq!(stream_id, 8);

    mock.send_to_server().await.unwrap();

    mock.send_to_client().await.unwrap();

    let send_data = b"hello world";

    mock.client
        .stream_send(stream_id, send_data, true)
        .await
        .unwrap();

    mock.send_to_server().await.unwrap();

    let server_conn = mock.listener_incoming.next().await.unwrap();

    assert_eq!(server_conn.stream_open(true).await.unwrap(), 5);
    assert_eq!(server_conn.stream_open(true).await.unwrap(), 9);

    assert_eq!(server_conn.stream_accept().await.unwrap(), stream_id);

    let mut buf = vec![0; 1024];

    let (read_size, fin) = server_conn.stream_recv(stream_id, &mut buf).await.unwrap();

    assert_eq!(send_data, &buf[..read_size]);
    assert!(fin);
}

#[futures_test::test]
async fn verify_client_cert() {
    init::init();

    let mut mock = MockQuic::new().await.unwrap();

    let stream_id = mock.client.stream_open(true).await.unwrap();

    mock.send_to_server().await.unwrap();

    // connection is established now.
    mock.send_to_client().await.unwrap();

    mock.client
        .stream_send(stream_id, b"hello", false)
        .await
        .unwrap();

    mock.send_to_server().await.unwrap();

    let server_conn = mock.listener_incoming.next().await.unwrap();

    assert!(server_conn.to_inner_conn().await.peer_cert().is_some());
}

#[futures_test::test]
async fn test_stream_stopped_by_server() {
    init::init();

    let mut mock = MockQuic::new().await.unwrap();

    let stream_id = mock.client.stream_open(true).await.unwrap();

    assert_eq!(stream_id, 4);

    let stream_id = mock.client.stream_open(true).await.unwrap();

    assert_eq!(stream_id, 8);

    mock.send_to_server().await.unwrap();

    mock.send_to_client().await.unwrap();

    let send_data = b"hello world";

    mock.client
        .stream_send(stream_id, send_data, false)
        .await
        .unwrap();

    mock.send_to_server().await.unwrap();

    let server_conn = mock.listener_incoming.next().await.unwrap();

    assert_eq!(server_conn.stream_open(true).await.unwrap(), 5);
    assert_eq!(server_conn.stream_open(true).await.unwrap(), 9);

    assert_eq!(server_conn.stream_accept().await.unwrap(), stream_id);

    let mut buf = vec![0; 1024];

    let (read_size, fin) = server_conn.stream_recv(stream_id, &mut buf).await.unwrap();

    assert_eq!(send_data, &buf[..read_size]);
    assert!(!fin);

    server_conn.stream_close(stream_id).await.unwrap();

    mock.send_to_client().await.unwrap();

    mock.client
        .stream_close(stream_id)
        .await
        .expect_err("Stopped by server");
}

#[futures_test::test]
async fn test_stream_stopped_by_client() {
    init::init();

    let mut mock = MockQuic::new().await.unwrap();

    let stream_id = mock.client.stream_open(true).await.unwrap();

    assert_eq!(stream_id, 4);

    let stream_id = mock.client.stream_open(true).await.unwrap();

    assert_eq!(stream_id, 8);

    mock.send_to_server().await.unwrap();

    mock.send_to_client().await.unwrap();

    let send_data = b"hello world";

    mock.client
        .stream_send(stream_id, send_data, true)
        .await
        .unwrap();

    mock.send_to_server().await.unwrap();

    let server_conn = mock.listener_incoming.next().await.unwrap();

    assert_eq!(server_conn.stream_open(true).await.unwrap(), 5);
    assert_eq!(server_conn.stream_open(true).await.unwrap(), 9);

    assert_eq!(server_conn.stream_accept().await.unwrap(), stream_id);

    let mut buf = vec![0; 1024];

    let (read_size, fin) = server_conn.stream_recv(stream_id, &mut buf).await.unwrap();

    assert_eq!(send_data, &buf[..read_size]);
    assert!(fin);

    server_conn.stream_close(stream_id).await.unwrap();

    mock.send_to_client().await.unwrap();

    mock.client.stream_close(stream_id).await.unwrap();
}
