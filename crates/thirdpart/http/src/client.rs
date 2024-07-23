use futures::{
    io::{self, Cursor},
    AsyncRead, AsyncWrite, Future,
};
use http::{Request, Response};

use crate::{
    parse::{BodyReader, Responser},
    writer::RequestWriter,
};

/// extension trait for the http "send" method added to the "AsyncWrite + AsyncRead + Unpin" object
pub trait HttpClient: AsyncWrite + AsyncRead + Send + Unpin {
    fn send<B>(
        mut self,
        request: http::Request<B>,
    ) -> impl Future<Output = io::Result<Response<BodyReader<Self>>>>
    where
        B: AsyncRead + Send + Unpin,
        Self: Sized,
    {
        async move {
            let writer = RequestWriter::new(&mut self);

            writer.write_with_stream_body(request).await?;

            let response = Responser::new(self).parse().await?;

            Ok(response)
        }
    }
}

impl<T: AsyncWrite + AsyncRead + Send + Unpin> HttpClient for T {}

/// A trait exntend [`Request`](http::Request) object with `send` and `send_with` methods.
pub trait HttpRequestWithStreamBodySend {
    fn send_with<T>(
        self,
        transport: &mut T,
    ) -> impl Future<Output = io::Result<Response<BodyReader<&mut T>>>>
    where
        T: AsyncWrite + AsyncRead + Send + Unpin;

    #[cfg(feature = "with_rasi")]
    #[cfg_attr(docsrs, doc(feature = "with_rasi"))]
    fn send<O: AsRef<crate::client::rasi::SendOptions>>(
        self,
        ops: O,
    ) -> impl Future<Output = io::Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>>;
}

impl<B> HttpRequestWithStreamBodySend for Request<B>
where
    B: AsyncRead + Send + Unpin,
{
    fn send_with<T>(
        self,
        transport: &mut T,
    ) -> impl Future<Output = io::Result<Response<BodyReader<&mut T>>>>
    where
        T: AsyncWrite + AsyncRead + Send + Unpin,
    {
        transport.send(self)
    }

    #[cfg(feature = "with_rasi")]
    #[cfg_attr(docsrs, doc(feature = "with_rasi"))]
    fn send<O: AsRef<crate::client::rasi::SendOptions>>(
        self,
        ops: O,
    ) -> impl Future<Output = io::Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>>
    {
        rasi::send(self, ops)
    }
}

/// A trait exntend [`Request`](http::Request) object with `send` and `send_with` methods.
pub trait HttpRequestSend {
    fn send_with<T>(
        self,
        transport: &mut T,
    ) -> impl Future<Output = io::Result<Response<BodyReader<&mut T>>>>
    where
        T: AsyncWrite + AsyncRead + Send + Unpin;

    #[cfg(feature = "with_rasi")]
    #[cfg_attr(docsrs, doc(feature = "with_rasi"))]
    fn send<O: AsRef<crate::client::rasi::SendOptions>>(
        self,
        ops: O,
    ) -> impl Future<Output = io::Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>>;
}

impl<B> HttpRequestSend for Request<B>
where
    B: AsRef<[u8]> + Send + Unpin,
{
    fn send_with<T>(
        self,
        transport: &mut T,
    ) -> impl Future<Output = io::Result<Response<BodyReader<&mut T>>>>
    where
        T: AsyncWrite + AsyncRead + Send + Unpin,
    {
        let (parts, body) = self.into_parts();

        let request = Request::from_parts(parts, Cursor::new(body));

        transport.send(request)
    }

    #[cfg(feature = "with_rasi")]
    #[cfg_attr(docsrs, doc(feature = "with_rasi"))]
    fn send<O: AsRef<crate::client::rasi::SendOptions>>(
        self,
        ops: O,
    ) -> impl Future<Output = io::Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>>
    {
        let (parts, body) = self.into_parts();

        let request = Request::from_parts(parts, Cursor::new(body));

        rasi::send(request, ops)
    }
}

#[cfg(feature = "with_rasi")]
#[cfg_attr(docsrs, doc(feature = "with_rasi"))]
pub mod rasi {

    use std::{
        io,
        net::{SocketAddr, ToSocketAddrs},
        path::{Path, PathBuf},
    };

    use futures::AsyncRead;
    use futures_boring::{
        connect,
        ssl::{SslConnector, SslMethod},
    };

    use http::uri::Scheme;
    use rasi::net::TcpStream;

    use http::Response;

    use crate::parse::BodyReader;

    #[derive(Debug, Default)]
    pub struct SendOptions {
        raddrs: Option<Vec<SocketAddr>>,
        server_name: Option<String>,
        ca_file: Option<PathBuf>,
    }

    impl AsRef<SendOptions> for SendOptions {
        fn as_ref(&self) -> &SendOptions {
            self
        }
    }

    impl SendOptions {
        /// Rewrite http request's host:port fields and send request to the specified `raddrs`.
        pub fn redirect<R: ToSocketAddrs>(mut self, raddrs: R) -> io::Result<Self> {
            self.raddrs = Some(
                raddrs
                    .to_socket_addrs()
                    .map(|iter| iter.collect::<Vec<_>>())?,
            );

            Ok(self)
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
    }

    /// Create a new http client for a specific [`Request`](http::Request), and process http request.
    pub async fn send<B, O: AsRef<SendOptions>>(
        request: http::Request<B>,
        ops: O,
    ) -> std::io::Result<Response<BodyReader<Box<dyn AsyncRead + Send + Unpin>>>>
    where
        B: AsyncRead + Send + Unpin,
    {
        let scheme = request.uri().scheme().ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unspecified request scheme",
        ))?;

        let host = request.uri().host().ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unspecified request uri",
        ))?;

        let port =
            request
                .uri()
                .port_u16()
                .unwrap_or_else(|| if scheme == &Scheme::HTTP { 80 } else { 440 });

        let ops = ops.as_ref();

        let raddrs = if let Some(raddrs) = &ops.raddrs {
            raddrs.to_owned()
        } else {
            vec![format!("{}:{}", host, port,)
                .parse()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?]
        };

        use super::HttpRequestWithStreamBodySend;

        if scheme == &Scheme::HTTP {
            let mut transport = TcpStream::connect(raddrs.as_slice()).await?;
            let response = request.send_with(&mut transport).await?;

            let (parts, body) = response.into_parts();

            let (bytes, _) = body.into_parts();

            let body: BodyReader<Box<dyn AsyncRead + Send + Unpin>> =
                BodyReader::new(bytes, Box::new(transport));

            let response = Response::from_parts(parts, body);

            return Ok(response);
        } else {
            let stream = TcpStream::connect(raddrs.as_slice()).await?;

            let mut config = SslConnector::builder(SslMethod::tls())
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

            if let Some(ca_file) = ops.ca_file.to_owned() {
                log::trace!("load trust root ca: {:?}", ca_file);

                config
                    .set_ca_file(ca_file)
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
            }

            let config = config.build().configure().unwrap();

            let mut transport = connect(config, host, stream)
                .await
                .map_err(|err| io::Error::new(io::ErrorKind::ConnectionRefused, err))?;

            let response = request.send_with(&mut transport).await?;

            let (parts, body) = response.into_parts();

            let (bytes, _) = body.into_parts();

            let body: BodyReader<Box<dyn AsyncRead + Send + Unpin>> =
                BodyReader::new(bytes, Box::new(transport));

            let response = Response::from_parts(parts, body);

            return Ok(response);
        }
    }
}
