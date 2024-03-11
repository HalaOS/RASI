use std::{net::SocketAddr, path::Path, sync::Arc};

use futures::{AsyncReadExt, AsyncWriteExt};
use rasi::{
    executor::spawn,
    net::{TcpListener, TcpStream},
};
use rasi_ext::net::tls::{
    accept, connect, SslAcceptor, SslConnector, SslFiletype, SslMethod, SslStream,
};

mod init;

async fn create_echo_server() -> SocketAddr {
    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    acceptor
        .set_private_key_file(root_path.join("cert/server.key"), SslFiletype::PEM)
        .unwrap();
    acceptor
        .set_certificate_chain_file(root_path.join("cert/server.crt"))
        .unwrap();

    acceptor.check_private_key().unwrap();
    let acceptor = Arc::new(acceptor.build());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let raddr = listener.local_addr().unwrap();

    spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let stream = accept(&acceptor, stream).await.unwrap();
                    spawn(handle_echo_stream(stream));
                }
                Err(_) => break,
            }
        }
    });

    raddr
}

async fn handle_echo_stream(mut stream: SslStream<TcpStream>) {
    let mut buf = vec![0; 1370];

    loop {
        let read_size = stream.read(&mut buf).await.unwrap();

        if read_size == 0 {
            break;
        }

        stream.write_all(&buf[..read_size]).await.unwrap();
    }
}

#[futures_test::test]
async fn test_echo() {
    init::init();

    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let raddr = create_echo_server().await;

    let stream = TcpStream::connect(raddr).await.unwrap();

    let mut config = SslConnector::builder(SslMethod::tls()).unwrap();

    config
        .set_ca_file(root_path.join("cert/rasi_ca.pem"))
        .unwrap();

    let config = config.build().configure().unwrap();

    let mut stream = connect(config, "rasi.quic", stream).await.unwrap();

    for i in 0..10000 {
        let send_data = format!("Hello world, {}", i);

        stream.write(send_data.as_bytes()).await.unwrap();

        let mut buf = [0; 1024];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], send_data.as_bytes());
    }
}
