use std::{fmt::Debug, task::Poll};

use futures::{
    io::{BufReader, Lines},
    stream::{once, BoxStream},
    AsyncBufReadExt, AsyncRead, AsyncReadExt, Stream, StreamExt,
};
use http::{
    header::{CONTENT_LENGTH, TRANSFER_ENCODING},
    HeaderMap,
};

#[derive(Debug, thiserror::Error)]
pub enum BodyReaderError {
    #[error("Parse CONTENT_LENGTH header with error: {0}")]
    ParseContentLength(String),

    #[error("Parse TRANSFER_ENCODING header with error: {0}")]
    ParseTransferEncoding(String),

    #[error("CONTENT_LENGTH or TRANSFER_ENCODING not found.")]
    UnsporTransferEncoding,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type BodyReaderResult<T> = Result<T, BodyReaderError>;

/// The sender to send http body data to peer.
pub struct BodyReader {
    length: Option<usize>,
    stream: BoxStream<'static, std::io::Result<Vec<u8>>>,
}

impl Debug for BodyReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BodyReader, length={:?}", self.length)
    }
}

impl From<Vec<u8>> for BodyReader {
    fn from(value: Vec<u8>) -> Self {
        Self {
            length: Some(value.len()),
            stream: Box::pin(once(async move { Ok(value) })),
        }
    }
}

impl From<&[u8]> for BodyReader {
    fn from(value: &[u8]) -> Self {
        value.to_owned().into()
    }
}

impl From<&str> for BodyReader {
    fn from(value: &str) -> Self {
        value.as_bytes().into()
    }
}

impl From<String> for BodyReader {
    fn from(value: String) -> Self {
        value.as_bytes().into()
    }
}

impl BodyReader {
    /// Create a new `BodySender` instance from `stream`
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = std::io::Result<Vec<u8>>> + Send + Unpin + 'static,
    {
        Self {
            length: None,
            stream: Box::pin(stream),
        }
    }

    /// Return true if the underlying data is a stream
    pub fn len(&self) -> Option<usize> {
        self.length
    }

    /// Parse headers and generate property `BodyReader`.
    pub async fn parse<R>(headers: &HeaderMap, mut read: R) -> BodyReaderResult<Self>
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        // TRANSFER_ENCODING has higher priority
        if let Some(transfer_encoding) = headers.get(TRANSFER_ENCODING) {
            let transfer_encoding = transfer_encoding
                .to_str()
                .map_err(|err| BodyReaderError::ParseTransferEncoding(err.to_string()))?;

            if transfer_encoding != "chunked" {
                return Err(BodyReaderError::ParseTransferEncoding(format!(
                    "Unsupport TRANSFER_ENCODING: {}",
                    transfer_encoding
                )));
            }

            return Ok(Self::from_stream(ChunkedBodyStream::from(read)));
        }

        if let Some(content_length) = headers.get(CONTENT_LENGTH) {
            let content_length = content_length
                .to_str()
                .map_err(|err| BodyReaderError::ParseContentLength(err.to_string()))?;

            let content_length = usize::from_str_radix(content_length, 10)
                .map_err(|err| BodyReaderError::ParseContentLength(err.to_string()))?;

            let mut buf = vec![0u8; content_length];

            read.read_exact(&mut buf).await?;

            return Ok(buf.into());
        }

        Ok(Self::from(vec![]))
    }
}

impl Stream for BodyReader {
    type Item = std::io::Result<Vec<u8>>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(cx)
    }
}

struct ChunkedBodyStream<R> {
    lines: Lines<BufReader<R>>,
    chunk_len: Option<usize>,
}

impl<R> From<R> for ChunkedBodyStream<R>
where
    R: AsyncRead + Unpin,
{
    fn from(value: R) -> Self {
        Self {
            lines: BufReader::new(value).lines(),
            chunk_len: None,
        }
    }
}

impl<R> Stream for ChunkedBodyStream<R>
where
    R: AsyncRead + Unpin,
{
    type Item = std::io::Result<Vec<u8>>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        loop {
            if let Some(mut len) = self.chunk_len {
                match self.lines.poll_next_unpin(cx) {
                    Poll::Ready(Some(Ok(buf))) => {
                        if buf.len() > len {
                            return Poll::Ready(Some(Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "chunck data overflow",
                            ))));
                        }

                        len -= buf.len();

                        if len == 0 {
                            self.chunk_len.take();
                        } else {
                            self.chunk_len = Some(len);
                        }

                        return Poll::Ready(Some(Ok(buf.into_bytes())));
                    }
                    poll => return poll.map_ok(|s| s.into_bytes()),
                }
            } else {
                match self.lines.poll_next_unpin(cx) {
                    Poll::Ready(Some(Ok(line))) => match usize::from_str_radix(&line, 16) {
                        Ok(len) => {
                            // body last chunk.
                            if len == 0 {
                                return Poll::Ready(None);
                            }

                            self.chunk_len = Some(len);
                            continue;
                        }
                        Err(err) => {
                            return Poll::Ready(Some(Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Parse chunck length with error: {}", err),
                            ))))
                        }
                    },
                    poll => return poll.map_ok(|s| s.into_bytes()),
                }
            }
        }
    }
}
