//! Utilities for http server-side program.
//!
//!

use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};

use crate::net::tls::{SslAcceptor, SslStream};
use futures::{AsyncReadExt, Future};
use http::{Request, Response, StatusCode};
use rasi::{
    executor::spawn,
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    net::{TcpListener, TcpStream, TcpStreamRead, TcpStreamWrite},
    time::TimeoutExt,
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
    timeout: Duration,
    mut handler: H,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin + 'static,
    H: FnMut(Request<BodyReader<R>>, ResponseWriter<W>) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = io::Result<()>> + Send,
{
    let response_writer = ResponseWriter::new(write);

    let request = match Requester::new(read).parse().timeout(timeout).await {
        Some(Ok(request)) => request,
        Some(Err(err)) => {
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
        None => {
            log::error!("{}, read/parse request timeout", label.unwrap_or("Unknown"));
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

/// The http server builder.
pub struct HttpServer {
    /// config to start with tcp listener.
    listener: Option<TcpListener>,
    /// config to start with local bound socket addresses.
    laddrs: Option<io::Result<Vec<SocketAddr>>>,
    /// The reading timeout duration for newly incoming http request, the default value is 30s.
    timeout: Duration,
}

impl Default for HttpServer {
    fn default() -> Self {
        Self {
            listener: None,
            laddrs: None,
            timeout: Duration::from_secs(30),
        }
    }
}

impl HttpServer {
    /// Create new http server builder with local binding addresses.
    pub fn bind<A: ToSocketAddrs>(laddrs: A) -> Self {
        let laddrs = laddrs
            .to_socket_addrs()
            .map(|iter| iter.collect::<Vec<_>>());

        Self {
            laddrs: Some(laddrs),
            ..Default::default()
        }
    }

    /// Create new http server builder with external [`TcpListener`].
    pub fn on(listener: TcpListener) -> Self {
        Self {
            listener: Some(listener),
            ..Default::default()
        }
    }

    /// Set the reading timeout duration for newly incoming http request.
    ///
    /// The default value is 30s.
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Convert [`HttpServer`] into [`HttpSslServer`] with provided [`ssl_acceptor`](SslAcceptor)
    pub fn with_ssl(self, ssl_acceptor: SslAcceptor) -> HttpSslServer {
        HttpSslServer {
            http_server: self,
            ssl_acceptor,
        }
    }

    /// Consume self and create http server [`listener`](TcpListener)
    ///
    /// On success, returns created [`listener`](TcpListener) and the `reading timeout duration`.
    pub async fn create(self) -> io::Result<(TcpListener, Duration)> {
        if let Some(listener) = self.listener {
            Ok((listener, self.timeout))
        } else {
            // Safety: the [`HttpServer`] can be created by `bind` or `on` functions only.
            let laddrs = self.laddrs.unwrap()?;

            TcpListener::bind(laddrs.as_slice())
                .await
                .map(|listener| (listener, self.timeout))
        }
    }

    /// Consume self and start http server with provided request `handler`.
    pub async fn serve<H, Fut>(self, handler: H) -> io::Result<()>
    where
        H: FnMut(Request<BodyReader<TcpStreamRead>>, ResponseWriter<TcpStreamWrite>) -> Fut
            + Send
            + Sync
            + Clone
            + 'static,
        Fut: Future<Output = io::Result<()>> + Send,
    {
        let (listener, timeout) = self.create().await?;

        loop {
            let (stream, raddr) = listener.accept().await?;

            log::trace!("Http request from {:?}", raddr);

            let (read, write) = stream.split();

            let handler = handler.clone();

            let label = format!("Http request from {:?}", raddr);

            spawn(async move {
                match serve(Some(&label), read, write, timeout, handler).await {
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
}

/// The https server configuration builder, which contains a http server configuration within.
pub struct HttpSslServer {
    http_server: HttpServer,
    ssl_acceptor: SslAcceptor,
}

impl HttpSslServer {
    /// Consume self and start https server with provided request `handler`.
    pub async fn serve<H, Fut>(self, handler: H) -> io::Result<()>
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
        let (listener, timeout) = self.http_server.create().await?;

        let ssl_acceptor = Arc::new(self.ssl_acceptor);

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

                let label = format!("Https request from {:?}", raddr);

                match serve(Some(&label), read, write, timeout, handler).await {
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
}
