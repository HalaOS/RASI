use std::{
    path::Path,
    sync::{Once, OnceLock},
};

use futures::{executor::ThreadPool, Future, TryStreamExt};
use futures_boring::{
    ssl::{SslAcceptor, SslFiletype, SslMethod},
    SslListener,
};
use futures_http::{
    client::rasio::{HttpClient, HttpClientOptions},
    server::HttpServer,
    writer::ResponseWriter,
};
use http::{Request, Response, StatusCode};
use rasi::net::TcpListener;
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

#[cfg_attr(docsrs, doc(feature = "with_rasi"))]
#[futures_test::test]
async fn test_http() {
    init();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let raddr = listener.local_addr().unwrap();

    spawn(async move {
        let server = HttpServer::on(Some("http_test"), listener);

        let mut incoming = server.into_incoming();

        while let Some((req, mut resp)) = incoming.try_next().await.unwrap() {
            assert_eq!(req.uri().path(), "/hello");

            resp.write_http_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body("hello world")
                    .unwrap(),
            )
            .await
            .unwrap();
        }
    });

    let response = Request::get(format!("http://{:?}/hello", raddr))
        .body("")
        .unwrap()
        .send(HttpClientOptions::new())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let (_, body) = response.into_parts();

    assert_eq!(body.into_bytes(1024).await.unwrap(), "hello world");
}

#[cfg_attr(docsrs, doc(feature = "with_rasi"))]
#[futures_test::test]
async fn test_https() {
    init();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let raddr = listener.local_addr().unwrap();

    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    acceptor
        .set_private_key_file(root_path.join("../../../cert/server.key"), SslFiletype::PEM)
        .unwrap();
    acceptor
        .set_certificate_chain_file(root_path.join("../../../cert/server.crt"))
        .unwrap();

    acceptor.check_private_key().unwrap();

    let acceptor = acceptor.build();

    let listener = SslListener::on(listener, acceptor);

    spawn(async move {
        let server = HttpServer::on(Some("test_https"), listener.into_incoming());

        let mut incoming = server.into_incoming();

        while let Some((req, mut resp)) = incoming.try_next().await.unwrap() {
            assert_eq!(req.uri().path(), "/hello");

            resp.write_http_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body("hello world")
                    .unwrap(),
            )
            .await
            .unwrap();
        }
    });

    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let ca_file = root_path.join("../../../cert/rasi_ca.pem");

    let response = Request::get(format!("https://rasi.quic/hello"))
        .body("")
        .unwrap()
        .send(
            HttpClientOptions::new()
                .redirect(raddr)
                .with_ca_file(ca_file),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let (_, body) = response.into_parts();

    assert_eq!(body.into_bytes(1024,).await.unwrap(), "hello world");
}
