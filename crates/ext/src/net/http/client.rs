//! Utilities for http client program.

use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
    pin::Pin,
    time::Duration,
};

use boring::ssl::{SslConnector, SslMethod};
use futures::{AsyncRead, AsyncWrite};
use http::{uri::Scheme, Request, Response};
use rasi::{net::TcpStream, time::TimeoutExt};

use crate::net::{
    http::{parse::Responser, writer::RequestWriter},
    tls::{connect, SslStream},
};

use super::parse::BodyReader;

pub enum HttpClientWrite {
    TcpStream(TcpStream),
    TlsStream(SslStream<TcpStream>),
}

impl Into<HttpClientRead> for HttpClientWrite {
    fn into(self) -> HttpClientRead {
        match self {
            HttpClientWrite::TcpStream(stream) => HttpClientRead::TcpStream(stream),
            HttpClientWrite::TlsStream(stream) => HttpClientRead::TlsStream(stream),
        }
    }
}

impl AsyncWrite for HttpClientWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match &mut *self {
            Self::TcpStream(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::TlsStream(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match &mut *self {
            Self::TcpStream(stream) => Pin::new(stream).poll_flush(cx),
            Self::TlsStream(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match &mut *self {
            Self::TcpStream(stream) => Pin::new(stream).poll_close(cx),
            Self::TlsStream(stream) => Pin::new(stream).poll_close(cx),
        }
    }
}

pub enum HttpClientRead {
    TcpStream(TcpStream),
    TlsStream(SslStream<TcpStream>),
}

impl AsyncRead for HttpClientRead {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match &mut *self {
            HttpClientRead::TcpStream(stream) => Pin::new(stream).poll_read(cx, buf),
            HttpClientRead::TlsStream(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

/// A extension trait for http [`Request`] builder.
pub trait HttpRequestSend {
    type Body;

    fn sender(self) -> HttpRequestSender<Self::Body>;
}

/// A builder to create a task to send http request.
#[must_use = "Must call response function to invoke real sending action."]
pub struct HttpRequestSender<T> {
    request: http::Result<Request<T>>,
    timeout: Duration,
    raddrs: Option<io::Result<Vec<SocketAddr>>>,
    server_name: Option<String>,
    ca_file: Option<PathBuf>,
}

impl<T> HttpRequestSender<T> {
    /// Create `HttpRequestSender` with the default configuration other than provided [`Request`].
    pub fn new(request: http::Result<Request<T>>) -> Self {
        Self {
            request,
            timeout: Duration::from_secs(30),
            raddrs: None,
            server_name: None,
            ca_file: None,
        }
    }

    /// Set the request send / response header recv timeout. the default value is `30s`.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Rewrite http request's host:port fields and send request to the specified `raddrs`.
    pub fn redirect<R: ToSocketAddrs>(mut self, raddrs: R) -> Self {
        self.raddrs = Some(
            raddrs
                .to_socket_addrs()
                .map(|iter| iter.collect::<Vec<_>>()),
        );

        self
    }

    /// Set remote server's server name, this option will rewrite request's host field.
    pub fn with_server_name(mut self, server_name: &str) -> Self {
        self.server_name = Some(server_name.to_string());

        self
    }

    /// Set the server verification ca file, this is useful for self signed server.
    pub fn with_ca_file<P: AsRef<Path>>(mut self, ca_file: P) -> Self {
        self.ca_file = Some(ca_file.as_ref().to_path_buf());
        self
    }

    /// Consume self and create new [`HttpClientWrite`].
    ///
    /// On success, The response wait `timeout` and the original [`Request`] are returned together.
    pub async fn create(self) -> io::Result<(Request<T>, HttpClientWrite, Duration)> {
        let request = self
            .request
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

        let scheme = request.uri().scheme().ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unspecified request scheme",
        ))?;

        let host = request.uri().host().ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unspecified request uri",
        ))?;

        let port =
            request.uri().port_u16().unwrap_or_else(
                || {
                    if scheme == &Scheme::HTTP {
                        80
                    } else {
                        440
                    }
                },
            );

        let raddr = if let Some(raddr) = self.raddrs {
            raddr?
        } else {
            vec![format!("{}:{}", host, port,)
                .parse()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?]
        };

        let stream = if scheme == &Scheme::HTTP {
            let stream = TcpStream::connect(raddr.as_slice()).await?;

            HttpClientWrite::TcpStream(stream)
        } else {
            let stream = TcpStream::connect(raddr.as_slice()).await?;

            let mut config = SslConnector::builder(SslMethod::tls())
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

            if let Some(ca_file) = self.ca_file {
                config
                    .set_ca_file(ca_file)
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
            }

            let config = config.build().configure().unwrap();

            let stream = connect(config, host, stream)
                .await
                .map_err(|err| io::Error::new(io::ErrorKind::ConnectionRefused, err))?;

            HttpClientWrite::TlsStream(stream)
        };

        Ok((request, stream, self.timeout))
    }

    /// Start a new
    pub async fn send(self) -> io::Result<Response<BodyReader<HttpClientRead>>>
    where
        T: AsRef<[u8]>,
    {
        let (request, mut stream, timeout) = self.create().await?;

        let writer = RequestWriter::new(&mut stream);

        match writer.write(request).timeout(timeout).await {
            Some(Ok(_)) => {}
            Some(Err(err)) => return Err(err),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "send http request timeout",
                ));
            }
        }

        let stream: HttpClientRead = stream.into();

        match Responser::new(stream).parse().timeout(timeout).await {
            Some(Ok(response)) => Ok(response),
            Some(Err(err)) => return Err(err.into()),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "recv http response header timeout",
                ));
            }
        }
    }

    pub async fn send_with_stream_body(self) -> io::Result<Response<BodyReader<HttpClientRead>>>
    where
        T: AsyncRead + Unpin,
    {
        let (request, mut stream, timeout) = self.create().await?;

        let writer = RequestWriter::new(&mut stream);

        match writer
            .write_with_stream_body(request)
            .timeout(timeout)
            .await
        {
            Some(Ok(_)) => {}
            Some(Err(err)) => return Err(err),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "send http request timeout",
                ));
            }
        }

        let stream: HttpClientRead = stream.into();

        match Responser::new(stream).parse().timeout(timeout).await {
            Some(Ok(response)) => Ok(response),
            Some(Err(err)) => return Err(err.into()),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "recv http response header timeout",
                ));
            }
        }
    }
}

impl<T> HttpRequestSend for http::Result<Request<T>>
where
    T: AsRef<[u8]>,
{
    type Body = T;

    fn sender(self) -> HttpRequestSender<Self::Body> {
        HttpRequestSender::new(self)
    }
}
