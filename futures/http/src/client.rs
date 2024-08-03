use std::{future::Future, io::Result};

use futures::{AsyncRead, AsyncWrite};
use http::{Request, Response};

use crate::body::BodyReader;
use crate::reader::HttpReader;
use crate::writer::HttpWriter;

/// An asynchronous *Client* to make http *Requests* with.
pub trait HttpSend {
    /// Sends the **Request** via `stream` and returns a future of [`Response`]
    fn send<S>(self, stream: S) -> impl Future<Output = Result<Response<BodyReader>>>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static;
}

impl HttpSend for Request<BodyReader> {
    fn send<S>(self, mut stream: S) -> impl Future<Output = Result<Response<BodyReader>>>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        async move {
            stream.write_request(self).await?;

            stream.read_response().await
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

    use futures_boring::{
        connect,
        ssl::{SslConnector, SslMethod},
    };
    use http::{uri::Scheme, Request, Response};
    use rasi::net::TcpStream;

    use crate::body::BodyReader;

    /// Options and flags which can be used to configure how a http client is opened.
    #[derive(Default, Debug, Clone)]
    pub struct HttpClientOptions {
        raddrs: Option<Vec<SocketAddr>>,
        server_name: Option<String>,
        ca_file: Option<PathBuf>,
        use_server_name_indication: bool,
    }

    impl HttpClientOptions {
        /// Create a `HttpClientOptionsBuilder` instance to build `HttpClientOptions`.
        pub fn new() -> HttpClientOptionsBuilder {
            HttpClientOptionsBuilder {
                ops: Ok(HttpClientOptions {
                    use_server_name_indication: true,
                    ..Default::default()
                }),
            }
        }

        async fn send(self, request: Request<BodyReader>) -> Result<Response<BodyReader>> {
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
                    443
                }
            });

            let raddrs = if let Some(raddrs) = &self.raddrs {
                raddrs.to_owned()
            } else {
                format!("{}:{}", host, port)
                    .to_socket_addrs()?
                    .collect::<Vec<_>>()
            };

            if scheme == &Scheme::HTTP {
                let transport = TcpStream::connect(raddrs.as_slice()).await?;

                return super::HttpSend::send(request, transport).await;
            } else {
                let stream = TcpStream::connect(raddrs.as_slice()).await?;

                let mut config = SslConnector::builder(SslMethod::tls_client())
                    .map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;

                if let Some(ca_file) = self.ca_file.to_owned() {
                    log::trace!("load trust root ca: {:?}", ca_file);

                    config
                        .set_ca_file(ca_file)
                        .map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
                }

                let mut config = config.build().configure().unwrap();

                config.set_use_server_name_indication(self.use_server_name_indication);

                let transport = connect(config, host, stream).await.map_err(|err| {
                    println!("{}", err);
                    Error::new(ErrorKind::ConnectionRefused, err)
                })?;

                return super::HttpSend::send(request, transport).await;
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

        /// Configures the use of Server Name Indication (SNI) when connecting.
        /// Defaults to true.
        pub fn set_use_server_name_indication(self, value: bool) -> Self {
            self.and_then(|mut ops| {
                ops.use_server_name_indication = value;

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

    /// An extension trait for [`Request`] with addition `send` method.
    pub trait HttpClient {
        /// Consume self and send [`Request`] via `stream` to peer.
        ///
        /// On success, returns [`Response`] from peer.
        fn send<Op>(self, ops: Op) -> impl Future<Output = Result<Response<BodyReader>>>
        where
            Op: TryInto<HttpClientOptions, Error = std::io::Error>;
    }

    impl HttpClient for Request<BodyReader> {
        fn send<Op>(self, ops: Op) -> impl Future<Output = Result<Response<BodyReader>>>
        where
            Op: TryInto<HttpClientOptions, Error = std::io::Error>,
        {
            async move {
                let ops: HttpClientOptions = ops.try_into()?;

                ops.send(self).await
            }
        }
    }
}
