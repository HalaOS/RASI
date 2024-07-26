//! Utilities for http server-side application.
//!
//!

use std::io::{Error, ErrorKind, Result};

use futures::{
    io::{ReadHalf, WriteHalf},
    AsyncRead, AsyncReadExt, AsyncWrite, Stream, StreamExt,
};
use http::{Request, Response, StatusCode};

use super::{
    parse::{BodyReader, Requester},
    writer::ResponseWriter,
};

pub struct HttpServer<I> {
    /// debug information.
    label: Option<String>,
    /// incoming http connection stream.
    incoming: I,
}

impl<I> HttpServer<I> {
    /// Start http server with provided http incoming connection stream.
    pub fn on(label: Option<&str>, incoming: I) -> Self {
        Self {
            label: label.map(|label| label.to_owned()),
            incoming,
        }
    }

    /// Accept new incoming http connection.
    pub async fn accept<S, E>(&mut self) -> Result<(Request<BodyReader<ReadHalf<S>>>, WriteHalf<S>)>
    where
        I: Stream<Item = std::result::Result<S, E>> + Unpin,
        S: AsyncRead + AsyncWrite + Send + Unpin,
        E: std::error::Error,
    {
        loop {
            match self
                .incoming
                .next()
                .await
                .ok_or(Error::new(ErrorKind::BrokenPipe, "http server shutdown."))?
            {
                Ok(stream) => {
                    let (read, mut write) = stream.split();

                    let request = match Requester::new(read).parse().await {
                        Ok(request) => request,
                        Err(err) => {
                            log::error!(
                                "{}, parse request error,{}",
                                self.label.as_deref().unwrap_or("Unknown"),
                                err
                            );

                            if let Err(err) = write
                                .write_http_response(
                                    Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(b"")
                                        .unwrap(),
                                )
                                .await
                            {
                                log::error!(
                                    "{}, send BAD_REQUEST to client,{}",
                                    self.label.as_deref().unwrap_or("Unknown"),
                                    err
                                );
                            }

                            continue;
                        }
                    };

                    return Ok((request, write));
                }
                Err(err) => return Err(Error::new(ErrorKind::Other, err.to_string())),
            }
        }
    }

    pub fn into_incoming<S, E>(
        self,
    ) -> impl Stream<Item = Result<(Request<BodyReader<ReadHalf<S>>>, WriteHalf<S>)>> + Unpin
    where
        I: Stream<Item = std::result::Result<S, E>> + Unpin,
        S: AsyncRead + AsyncWrite + Send + Unpin,
        E: std::error::Error,
    {
        Box::pin(futures::stream::unfold(self, |mut listener| async move {
            let res = listener.accept().await;
            Some((res, listener))
        }))
    }
}
