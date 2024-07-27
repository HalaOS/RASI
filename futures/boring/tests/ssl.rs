use std::{
    net::SocketAddr,
    path::Path,
    sync::{Once, OnceLock},
};

use futures::{executor::ThreadPool, AsyncReadExt, AsyncWriteExt, Future};

use rasi::net::{TcpListener, TcpStream};

use futures_boring::{connect, ssl, SslListener, SslStream};
use rasi_mio::{net::register_mio_network, timer::register_mio_timer};

fn spawn<Fut>(fut: Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    static THREAD_POOL: OnceLock<ThreadPool> = OnceLock::new();

    let thread_pool =
        THREAD_POOL.get_or_init(|| ThreadPool::builder().pool_size(10).create().unwrap());

    thread_pool.spawn_ok(fut)
}

fn init() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        register_mio_network();
        register_mio_timer();
    })
}

async fn create_echo_server() -> SocketAddr {
    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut acceptor = ssl::SslAcceptor::mozilla_intermediate(ssl::SslMethod::tls()).unwrap();

    acceptor
        .set_private_key_file(
            root_path.join("../../cert/server.key"),
            ssl::SslFiletype::PEM,
        )
        .unwrap();
    acceptor
        .set_certificate_chain_file(root_path.join("../../cert/server.crt"))
        .unwrap();

    acceptor.check_private_key().unwrap();
    let acceptor = acceptor.build();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let raddr = listener.local_addr().unwrap();

    let mut listener = SslListener::on(listener, acceptor);

    spawn(async move {
        loop {
            match listener.accept().await {
                Ok(stream) => {
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
    init();

    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let raddr = create_echo_server().await;

    let stream = TcpStream::connect(raddr).await.unwrap();

    let mut config = ssl::SslConnector::builder(ssl::SslMethod::tls()).unwrap();

    config
        .set_ca_file(root_path.join("../../cert/rasi_ca.pem"))
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
