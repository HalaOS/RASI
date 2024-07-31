use std::{
    future::Future,
    io::{Error, ErrorKind, Result},
};

use futures::{AsyncWrite, AsyncWriteExt, TryStreamExt};
use http::{
    header::{ToStrError, CONTENT_LENGTH, TRANSFER_ENCODING},
    HeaderValue, Request, Response,
};

use crate::body::BodyReader;

fn map_to_str_error(err: ToStrError) -> Error {
    Error::new(ErrorKind::InvalidData, err)
}

pub trait HttpWriter: AsyncWrite + Unpin {
    fn write_request(&mut self, request: Request<BodyReader>) -> impl Future<Output = Result<()>> {
        async move {
            let (mut parts, mut body) = request.into_parts();

            self.write_all(
                format!(
                    "{} {} {:?}\r\n",
                    parts.method,
                    parts.uri.path(),
                    parts.version
                )
                .as_bytes(),
            )
            .await?;

            if parts.headers.get(CONTENT_LENGTH).is_none()
                && parts.headers.get(TRANSFER_ENCODING).is_none()
            {
                if let Some(len) = body.len() {
                    parts.headers.insert(CONTENT_LENGTH, len.into());
                } else {
                    parts
                        .headers
                        .insert(TRANSFER_ENCODING, HeaderValue::from_static("chunked"));
                }
            }

            for (name, value) in &parts.headers {
                self.write_all(
                    format!(
                        "{}: {}\r\n",
                        name,
                        value.to_str().map_err(map_to_str_error)?
                    )
                    .as_bytes(),
                )
                .await?;
            }

            self.write_all(b"\r\n").await?;

            if let Some(len) = body.len() {
                let body = body
                    .try_next()
                    .await?
                    .expect("Ready fixed length body error");

                assert_eq!(body.len(), len);

                self.write_all(&body).await?;
            } else {
                while let Some(chunk) = body.try_next().await? {
                    self.write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                        .await?;

                    self.write_all(&chunk).await?;
                }

                self.write_all(b"0\r\n").await?;
            }

            Ok(())
        }
    }

    fn write_response(
        &mut self,
        response: Response<BodyReader>,
    ) -> impl Future<Output = Result<()>> {
        async move {
            let (parts, mut body) = response.into_parts();

            // write status line.
            self.write_all(format!("{:?} {}\r\n", parts.version, parts.status).as_bytes())
                .await?;

            for (name, value) in &parts.headers {
                self.write_all(
                    format!(
                        "{}: {}\r\n",
                        name,
                        value.to_str().map_err(map_to_str_error)?
                    )
                    .as_bytes(),
                )
                .await?;
            }

            if let Some(len) = body.len() {
                self.write_all(format!("{}: {}\r\n", CONTENT_LENGTH, len).as_bytes())
                    .await?;

                self.write_all(b"\r\n").await?;

                let body = body
                    .try_next()
                    .await?
                    .expect("Ready fixed length body error");

                self.write_all(&body).await?;
            } else {
                self.write_all(format!("{}: chunked\r\n", TRANSFER_ENCODING).as_bytes())
                    .await?;

                self.write_all(b"\r\n").await?;

                while let Some(chunk) = body.try_next().await? {
                    self.write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                        .await?;

                    self.write_all(&chunk).await?;
                }

                self.write_all(b"0\r\n\r\n").await?;
            }

            Ok(())
        }
    }
}

impl<T: AsyncWrite + Unpin> HttpWriter for T {}
