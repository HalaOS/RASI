//! Utilities for http server-side program.
//!
//!

use std::{io, net::ToSocketAddrs, sync::Arc};

use crate::net::tls::{SslAcceptor, SslStream};
use futures::{AsyncReadExt, Future};
use http::{Request, Response, StatusCode};
use rasi::{
    executor::spawn,
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    net::{TcpListener, TcpStream, TcpStreamRead, TcpStreamWrite},
};

use crate::net::tls::accept;

use super::{
    parse::{BodyReader, Requester},
    writer::ResponseWriter,
};

/// Parse the incoming http request via the [`read`](AsyncRead) and then, call the `handler` to process the request.
///
/// On success, the function write the `response` returns by `handler` into the [`write`](AsyncWrite) stream.
///
/// * If parse request error,  this function will write `400 Bad Request` as response.
/// * If the `handler` returns error, this function will write `500 Internal Server Error` as response.
pub async fn serve<R, W, H, Fut>(
    label: Option<&str>,
    read: R,
    write: W,
    mut handler: H,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin + 'static,
    H: FnMut(Request<BodyReader<R>>, ResponseWriter<W>) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = io::Result<()>> + Send,
{
    let response_writer = ResponseWriter::new(write);

    let request = match Requester::new(read).parse().await {
        Ok(request) => request,
        Err(err) => {
            log::error!(
                "{}, parse request error,{}",
                label.unwrap_or("Unknown"),
                err
            );

            response_writer
                .write(
                    Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(b"")
                        .unwrap(),
                )
                .await?;

            return Ok(());
        }
    };

    log::trace!("parsed request");

    match handler(request, response_writer).await {
        Ok(_) => Ok(()),
        Err(err) => {
            log::error!(
                "{}, internal server error, {}",
                label.unwrap_or("Unknown"),
                err
            );

            Ok(())
        }
    }
}

/// Start a http server with provided [`tcp listener`](TcpListener)
pub async fn http_serve_with<H, Fut>(listener: TcpListener, handler: H) -> io::Result<()>
where
    H: FnMut(Request<BodyReader<TcpStreamRead>>, ResponseWriter<TcpStreamWrite>) -> Fut
        + Send
        + Sync
        + Clone
        + 'static,
    Fut: Future<Output = io::Result<()>> + Send,
{
    // let ssl_acceptor = ssl_acceptor.map(|a| Arc::new(a));

    loop {
        let (stream, raddr) = listener.accept().await?;

        log::trace!("Http request from {:?}", raddr);

        let (read, write) = stream.split();

        let handler = handler.clone();

        spawn(async move {
            match serve(None, read, write, handler).await {
                Ok(_) => {
                    log::trace!("serve http request from {:?} success.", raddr);
                }
                Err(err) => {
                    log::error!("serve http request from {:?} failed, {}.", raddr, err);
                }
            }
        });
    }
}

/// Start a http server and listen on `laddrs`, this function will block until the server exits.
pub async fn http_serve<S: ToSocketAddrs, H, Fut>(laddrs: S, handler: H) -> io::Result<()>
where
    H: FnMut(Request<BodyReader<TcpStreamRead>>, ResponseWriter<TcpStreamWrite>) -> Fut
        + Send
        + Sync
        + Clone
        + 'static,
    Fut: Future<Output = io::Result<()>> + Send,
{
    let listener = TcpListener::bind(laddrs).await?;

    http_serve_with(listener, handler).await
}

/// Start a http server with provided [`tcp listener`](TcpListener)
pub async fn https_serve_with<H, Fut>(
    listener: TcpListener,
    ssl_acceptor: SslAcceptor,
    handler: H,
) -> io::Result<()>
where
    H: FnMut(
            Request<BodyReader<ReadHalf<SslStream<TcpStream>>>>,
            ResponseWriter<WriteHalf<SslStream<TcpStream>>>,
        ) -> Fut
        + Send
        + Sync
        + Clone
        + 'static,
    Fut: Future<Output = io::Result<()>> + Send,
{
    let ssl_acceptor = Arc::new(ssl_acceptor);

    loop {
        let (stream, raddr) = listener.accept().await?;

        log::trace!("Http request from {:?}", raddr);

        let handler = handler.clone();

        let ssl_acceptor = ssl_acceptor.clone();

        spawn(async move {
            let stream = accept(&ssl_acceptor, stream).await;

            let stream = match stream {
                Ok(stream) => stream,
                Err(err) => {
                    log::error!(
                        "serve http request from {:?}, tls handshake error, {}",
                        raddr,
                        err
                    );
                    return;
                }
            };

            let (read, write) = stream.split();

            match serve(None, read, write, handler).await {
                Ok(_) => {
                    log::trace!("serve http request from {:?} success.", raddr);
                }
                Err(err) => {
                    log::error!("serve http request from {:?} failed, {}.", raddr, err);
                }
            }
        });
    }
}

/// Start a http server and listen on `laddrs`, this function will block until the server exits.
pub async fn https_serve<S: ToSocketAddrs, H, Fut>(
    laddrs: S,
    ssl_acceptor: SslAcceptor,
    handler: H,
) -> io::Result<()>
where
    H: FnMut(
            Request<BodyReader<ReadHalf<SslStream<TcpStream>>>>,
            ResponseWriter<WriteHalf<SslStream<TcpStream>>>,
        ) -> Fut
        + Send
        + Sync
        + Clone
        + 'static,
    Fut: Future<Output = io::Result<()>> + Send,
{
    let listener = TcpListener::bind(laddrs).await?;

    https_serve_with(listener, ssl_acceptor, handler).await
}
