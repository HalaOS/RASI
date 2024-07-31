use std::{
    fmt::Write,
    future::Future,
    io::{Error, ErrorKind, Result},
    str::from_utf8,
};

use futures::{AsyncWrite, AsyncWriteExt, TryStreamExt};
use http::{
    header::{ToStrError, CONTENT_LENGTH, TRANSFER_ENCODING},
    Request, Response,
};

use crate::body::BodyReader;

fn map_to_str_error(err: ToStrError) -> Error {
    Error::new(ErrorKind::InvalidData, err)
}

pub trait HttpWriter: AsyncWrite + Unpin {
    fn write_request(&mut self, request: Request<BodyReader>) -> impl Future<Output = Result<()>> {
        async move {
            let (parts, mut body) = request.into_parts();

            let mut buf = String::new();

            write!(
                &mut buf,
                "{} {} {:?}\r\n",
                parts.method,
                parts.uri.path(),
                parts.version
            )
            .unwrap();

            // write status line.
            // self.write_all(status_line.as_bytes()).await?;

            for (name, value) in &parts.headers {
                write!(
                    &mut buf,
                    "{}: {}\r\n",
                    name,
                    value.to_str().map_err(map_to_str_error)?
                )
                .unwrap();
            }

            if let Some(len) = body.len() {
                write!(&mut buf, "{}: {}\r\n", CONTENT_LENGTH, len).unwrap();

                write!(&mut buf, "\r\n").unwrap();

                let body = body
                    .try_next()
                    .await?
                    .expect("Ready fixed length body error");

                let buf = buf + from_utf8(&body).unwrap();

                self.write_all(buf.as_bytes()).await?;

                println!("{}", buf);
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
