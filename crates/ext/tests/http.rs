use std::path::Path;

use boring::ssl::{SslAcceptor, SslFiletype, SslMethod};
use http::{Response, StatusCode};
use rasi::{executor::spawn, net::TcpListener};
use rasi_ext::net::http::{
    client::{req, RequestOps},
    server::{http_serve_with, https_serve_with},
    types::Request,
};

mod init;

#[futures_test::test]
async fn test_http() {
    init::init();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let raddr = listener.local_addr().unwrap();

    spawn(async move {
        http_serve_with(listener, |req, resp| async move {
            assert_eq!(req.uri().path(), "/hello");

            resp.write(
                Response::builder()
                    .status(StatusCode::OK)
                    .body("hello world")
                    .unwrap(),
            )
            .await?;

            Ok(())
        })
        .await
        .unwrap();
    });

    let response = req(
        Request::get(format!("http://{:?}/hello", raddr))
            .body("")
            .unwrap(),
        Default::default(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let (_, body) = response.into_parts();

    assert_eq!(body.into_bytes(1024).await.unwrap(), "hello world");
}

#[futures_test::test]
async fn test_https() {
    init::init();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let raddr = listener.local_addr().unwrap();

    spawn(async move {
        let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

        let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

        acceptor
            .set_private_key_file(root_path.join("cert/server.key"), SslFiletype::PEM)
            .unwrap();
        acceptor
            .set_certificate_chain_file(root_path.join("cert/server.crt"))
            .unwrap();

        acceptor.check_private_key().unwrap();

        https_serve_with(listener, acceptor.build(), |req, resp| async move {
            assert_eq!(req.uri().path(), "/hello");

            resp.write(
                Response::builder()
                    .status(StatusCode::OK)
                    .body("hello world")
                    .unwrap(),
            )
            .await?;

            Ok(())
        })
        .await
        .unwrap();
    });

    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let ca_file = root_path.join("cert/rasi_ca.pem");

    let response = req(
        Request::get(format!("https://rasi.quic/hello"))
            .body("")
            .unwrap(),
        RequestOps {
            raddr: Some(format!("{:?}", raddr)),
            ca_file: Some(&ca_file),
        },
    )
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let (_, body) = response.into_parts();

    assert_eq!(body.into_bytes(1024).await.unwrap(), "hello world");
}
