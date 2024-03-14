//! Utilities for http client program.

use std::{io, path::Path, pin::Pin};

use boring::ssl::{SslConnector, SslMethod};
use futures::AsyncRead;
use http::{uri::Scheme, Request, Response};
use rasi::net::TcpStream;

use crate::net::{
    http::{parse::Responser, writer::RequestWriter},
    tls::{connect, SslStream},
};

use super::parse::BodyReader;

pub enum HttpStreamRead {
    TcpStream(TcpStream),
    TlsStream(SslStream<TcpStream>),
}

impl AsyncRead for HttpStreamRead {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match &mut *self {
            HttpStreamRead::TcpStream(stream) => Pin::new(stream).poll_read(cx, buf),
            HttpStreamRead::TlsStream(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

#[derive(Default)]
pub struct RequestOps<'a> {
    pub raddr: Option<String>,
    pub ca_file: Option<&'a Path>,
}

/// Connect to `raddrs`, and process an http `request`.
///
/// On success, returns [`Response<BodyReader<TcpStreamRead>>`]
pub async fn req<'a, T>(
    request: Request<T>,
    ops: RequestOps<'a>,
) -> io::Result<Response<BodyReader<HttpStreamRead>>>
where
    T: AsRef<[u8]>,
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

    let raddr = if let Some(raddr) = ops.raddr {
        raddr
    } else {
        format!("{}:{}", host, port,)
    };

    let stream = if scheme == &Scheme::HTTP {
        let mut stream = TcpStream::connect(raddr).await?;

        let writer = RequestWriter::new(&mut stream);

        writer.write(request).await?;

        HttpStreamRead::TcpStream(stream)
    } else {
        let stream = TcpStream::connect(raddr).await?;

        let mut config = SslConnector::builder(SslMethod::tls())
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

        if let Some(ca_file) = ops.ca_file {
            config
                .set_ca_file(ca_file)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        }

        let config = config.build().configure().unwrap();

        let mut stream = connect(config, host, stream)
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::ConnectionRefused, err))?;

        let writer = RequestWriter::new(&mut stream);

        writer.write(request).await?;

        HttpStreamRead::TlsStream(stream)
    };

    Ok(Responser::new(stream).parse().await?)
}
