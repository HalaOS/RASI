//! Utilities for http client-side application.

use std::{future::Future, io::Result};

use futures::{AsyncRead, AsyncWrite};
use http::{Request, Response};

use crate::{
    parse::{BodyReader, Responser},
    writer::{RequestWithStreamBodyWriter, RequestWriter},
};

/// An extension trait for [`Request`](http::Request) with addition `send` method.
pub trait HttpWithStreamBodyClient {
    /// Consume self and send [`Request`](http::Request) via `stream` to peer.
    ///
    /// On success, returns [`Response`](http::Response) from peer.
    fn send<S>(self, stream: &mut S) -> impl Future<Output = Result<Response<BodyReader<&mut S>>>>
    where
        S: AsyncRead + AsyncWrite + Unpin;
}

impl<T> HttpWithStreamBodyClient for Request<T>
where
    T: AsyncRead + Unpin + 'static,
{
    fn send<S>(self, stream: &mut S) -> impl Future<Output = Result<Response<BodyReader<&mut S>>>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        async move {
            RequestWithStreamBodyWriter::write_http_request(stream, self).await?;

            let response = Responser::new(stream).parse().await?;

            Ok(response)
        }
    }
}

/// An extension trait for [`Request`](http::Request) with addition `send` method.
pub trait HttpClient {
    /// Consume self and send [`Request`](http::Request) via `stream` to peer.
    ///
    /// On success, returns [`Response`](http::Response) from peer.
    fn send<S>(self, stream: &mut S) -> impl Future<Output = Result<Response<BodyReader<&mut S>>>>
    where
        S: AsyncRead + AsyncWrite + Unpin;
}

impl<T> HttpClient for Request<T>
where
    T: AsRef<[u8]>,
{
    fn send<S>(self, stream: &mut S) -> impl Future<Output = Result<Response<BodyReader<&mut S>>>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        async move {
            RequestWriter::write_http_request(stream, self).await?;

            let response = Responser::new(stream).parse().await?;

            Ok(response)
        }
    }
}

#[cfg(feature = "with_rasi")]
pub mod rasio {
    use std::{
        future::Future,
        io::{Error, ErrorKind, Result},
        net::{SocketAddr, ToSocketAddrs},
        path::{Path, PathBuf},
    };

    use futures::{io::Cursor, AsyncRead};
    use futures_boring::{
        connect,
        ssl::{SslConnector, SslMethod},
    };
    use http::{uri::Scheme, Request, Response};
    use rasi::net::TcpStream;

    use crate::parse::BodyReader;

    /// Options and flags which can be used to configure how a http client is opened.
    #[derive(Default, Debug, Clone)]
    pub struct HttpClientOptions {
        raddrs: Option<Vec<SocketAddr>>,
        server_name: Option<String>,
        ca_file: Option<PathBuf>,
    }

    impl HttpClientOptions {
        /// Create a `HttpClientOptionsBuilder` instance to build `HttpClientOptions`.
        pub fn new() -> HttpClientOptionsBuilder {
            HttpClientOptionsBuilder {
                ops: Ok(HttpClientOptions::default()),
            }
        }

        async fn send<T>(
            self,
            request: Request<T>,
        ) -> Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>
        where
            T: AsyncRead + Unpin + 'static,
        {
            let scheme = request.uri().scheme().ok_or(Error::new(
                ErrorKind::InvalidInput,
                "Unspecified request scheme",
            ))?;

            let host = request.uri().host().ok_or(Error::new(
                ErrorKind::InvalidInput,
                "Unspecified request uri",
            ))?;

            let port = request.uri().port_u16().unwrap_or_else(|| {
                if scheme == &Scheme::HTTP {
                    80
                } else {
                    440
                }
            });

            let raddrs = if let Some(raddrs) = &self.raddrs {
                raddrs.to_owned()
            } else {
                vec![format!("{}:{}", host, port,)
                    .parse()
                    .map_err(|err| Error::new(ErrorKind::InvalidInput, err))?]
            };

            if scheme == &Scheme::HTTP {
                let mut transport = TcpStream::connect(raddrs.as_slice()).await?;
                let response =
                    super::HttpWithStreamBodyClient::send(request, &mut transport).await?;

                let (parts, body) = response.into_parts();

                let (bytes, _) = body.into_parts();

                let body: BodyReader<Box<dyn AsyncRead + Send + Unpin>> =
                    BodyReader::new(bytes, Box::new(transport));

                let response = Response::from_parts(parts, body);

                return Ok(response);
            } else {
                let stream = TcpStream::connect(raddrs.as_slice()).await?;

                let mut config = SslConnector::builder(SslMethod::tls())
                    .map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;

                if let Some(ca_file) = self.ca_file.to_owned() {
                    log::trace!("load trust root ca: {:?}", ca_file);

                    config
                        .set_ca_file(ca_file)
                        .map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
                }

                let config = config.build().configure().unwrap();

                let mut transport = connect(config, host, stream)
                    .await
                    .map_err(|err| Error::new(ErrorKind::ConnectionRefused, err))?;

                let response =
                    super::HttpWithStreamBodyClient::send(request, &mut transport).await?;

                let (parts, body) = response.into_parts();

                let (bytes, _) = body.into_parts();

                let body: BodyReader<Box<dyn AsyncRead + Send + Unpin>> =
                    BodyReader::new(bytes, Box::new(transport));

                let response = Response::from_parts(parts, body);

                return Ok(response);
            }
        }
    }

    impl TryInto<HttpClientOptions> for &HttpClientOptions {
        type Error = std::io::Error;

        fn try_into(self) -> std::result::Result<HttpClientOptions, Self::Error> {
            Ok(self.clone())
        }
    }

    /// A `HttpClientOptions` builder.
    pub struct HttpClientOptionsBuilder {
        ops: Result<HttpClientOptions>,
    }

    impl HttpClientOptionsBuilder {
        pub fn redirect<R: ToSocketAddrs>(self, raddrs: R) -> Self {
            self.and_then(|mut ops| {
                ops.raddrs = Some(
                    raddrs
                        .to_socket_addrs()
                        .map(|iter| iter.collect::<Vec<_>>())?,
                );
                Ok(ops)
            })
        }

        /// Set remote server's server name, this option will rewrite request's host field.
        pub fn with_server_name(self, server_name: &str) -> Self {
            self.and_then(|mut ops| {
                ops.server_name = Some(server_name.to_string());

                Ok(ops)
            })
        }

        /// Set the server verification ca file, this is useful for self signed server.
        pub fn with_ca_file<P: AsRef<Path>>(self, ca_file: P) -> Self {
            self.and_then(|mut ops| {
                ops.ca_file = Some(ca_file.as_ref().to_path_buf());

                Ok(ops)
            })
        }

        fn and_then<F>(self, func: F) -> Self
        where
            F: FnOnce(HttpClientOptions) -> Result<HttpClientOptions>,
        {
            HttpClientOptionsBuilder {
                ops: self.ops.and_then(func),
            }
        }
    }

    impl TryInto<HttpClientOptions> for HttpClientOptionsBuilder {
        type Error = std::io::Error;
        fn try_into(self) -> std::result::Result<HttpClientOptions, Self::Error> {
            self.ops
        }
    }

    /// An extension trait for [`Request`](http::Request) with addition `send` method.
    pub trait HttpWithStreamBodyClient {
        /// Consume self and send [`Request`](http::Request) via `stream` to peer.
        ///
        /// On success, returns [`Response`](http::Response) from peer.
        fn send<Op>(
            self,
            ops: Op,
        ) -> impl Future<Output = Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>>
        where
            Op: TryInto<HttpClientOptions, Error = std::io::Error>;
    }

    impl<T> HttpWithStreamBodyClient for Request<T>
    where
        T: AsyncRead + Unpin + 'static,
    {
        fn send<Op>(
            self,
            ops: Op,
        ) -> impl Future<Output = Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>>
        where
            Op: TryInto<HttpClientOptions, Error = std::io::Error>,
        {
            async move {
                let ops: HttpClientOptions = ops.try_into()?;

                ops.send(self).await
            }
        }
    }

    /// An extension trait for [`Request`](http::Request) with addition `send` method.
    pub trait HttpClient {
        /// Consume self and send [`Request`](http::Request) via `stream` to peer.
        ///
        /// On success, returns [`Response`](http::Response) from peer.
        fn send<Op>(
            self,
            ops: Op,
        ) -> impl Future<Output = Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>>
        where
            Op: TryInto<HttpClientOptions, Error = std::io::Error>;
    }

    impl<T> HttpClient for Request<T>
    where
        T: AsRef<[u8]>,
    {
        fn send<Op>(
            self,
            ops: Op,
        ) -> impl Future<Output = Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>>
        where
            Op: TryInto<HttpClientOptions, Error = std::io::Error>,
        {
            async move {
                let ops: HttpClientOptions = ops.try_into()?;

                let (parts, body) = self.into_parts();

                let body = Cursor::new(body.as_ref().to_owned());

                ops.send(Request::from_parts(parts, body)).await
            }
        }
    }
}
