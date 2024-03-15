use std::path::Path;

use boring::ssl::{SslAcceptor, SslFiletype, SslMethod};
use http::{Response, StatusCode};
use rasi::{executor::spawn, net::TcpListener};
use rasi_ext::net::http::{client::HttpRequestSend, server::HttpServer, types::Request};

mod init;

#[futures_test::test]
async fn test_http() {
    init::init();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let raddr = listener.local_addr().unwrap();

    spawn(async move {
        HttpServer::on(listener)
            .serve(|req, resp| async move {
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

    let response = Request::get(format!("http://{:?}/hello", raddr))
        .body("")
        .unwrap()
        .send()
        .response()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let (_, body) = response.into_parts();

    assert_eq!(body.into_bytes(1024, None).await.unwrap(), "hello world");
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

        HttpServer::on(listener)
            .with_ssl(acceptor.build())
            .serve(|req, resp| async move {
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

    let response = Request::get(format!("https://rasi.quic/hello"))
        .body("")
        .unwrap()
        .send()
        .send_to(raddr)
        .with_ca_file(ca_file)
        .response()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let (_, body) = response.into_parts();

    assert_eq!(body.into_bytes(1024, None).await.unwrap(), "hello world");
}
