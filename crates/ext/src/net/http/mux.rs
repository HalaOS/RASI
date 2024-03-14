//! This mod implement http server multiplexer.

use std::io;

use http::Request;
use rasi::io::{AsyncRead, AsyncWrite};
use std::future::Future;

use crate::net::http::parse::Requester;

use super::parse::BodyReader;

pub struct ResponseWriter<S> {
    #[allow(unused)]
    stream: S,
}

impl<S> ResponseWriter<S>
where
    S: AsyncWrite + Unpin,
{
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

pub trait Handler<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    type Fut: Future<Output = io::Result<()>>;

    fn handle(
        &mut self,
        request: Request<BodyReader<R>>,
        response: &mut ResponseWriter<W>,
    ) -> Self::Fut;
}

/// The http protocol multiplexer type.
#[derive(Default)]
pub struct ServerMux {}

impl ServerMux {
    /// Handle request packet received via the inbound stream.
    ///
    /// on successs, this function write response packet to the outbound stream.
    ///
    /// # Parameters
    /// - ***stream*** The bidirectional stream.
    pub async fn handle<R, W>(&self, read_half: R, write_half: W) -> io::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let _request = Requester::new(read_half).parse().await?;

        let _writer = ResponseWriter::new(write_half);

        todo!()
    }
}
