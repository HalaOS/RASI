//! Utilities to serialize http packets into a stream of bytes.
//!
use std::{future::Future, io};

use futures::io::{copy, AsyncRead, AsyncWrite, AsyncWriteExt, Cursor};
use http::{header::ToStrError, Request, Response};

fn map_to_str_error(err: ToStrError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

/// Writer for http request with stream body
pub trait RequestWithStreamBodyWriter: AsyncWrite + Unpin {
    /// write request packet into output stream.
    fn write_http_request<T>(
        &mut self,
        mut request: Request<T>,
    ) -> impl Future<Output = io::Result<()>>
    where
        T: AsyncRead + Unpin + 'static,
    {
        async move {
            // write status line.
            self.write_all(
                format!(
                    "{} {} {:?}\r\n",
                    request.method(),
                    request.uri(),
                    request.version()
                )
                .as_bytes(),
            )
            .await?;

            // write headers.
            for (name, value) in request.headers() {
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

            // write body.
            copy(request.body_mut(), self).await?;

            Ok(())
        }
    }
}

impl<T: AsyncWrite + Unpin> RequestWithStreamBodyWriter for T {}

/// Writer for http request with stream body
pub trait RequestWriter: AsyncWrite + Unpin {
    /// write request packet into output stream.
    fn write_http_request<T>(&mut self, request: Request<T>) -> impl Future<Output = io::Result<()>>
    where
        T: AsRef<[u8]>,
        Self: Sized,
    {
        async move {
            let (parts, body) = request.into_parts();

            let body = Cursor::new(body.as_ref().to_owned());

            RequestWithStreamBodyWriter::write_http_request(self, Request::from_parts(parts, body))
                .await
        }
    }
}

impl<T: AsyncWrite + Unpin> RequestWriter for T {}

/// Writer for http response with stream body
pub trait ResponseWithStreamBodyWriter: AsyncWrite + Unpin {
    /// write request packet into output stream.
    fn write_http_response<T>(
        &mut self,
        mut response: Response<T>,
    ) -> impl Future<Output = io::Result<()>>
    where
        T: AsyncRead + Unpin + 'static,
    {
        async move {
            // write status line.
            self.write_all(
                format!("{:?} {}\r\n", response.version(), response.status()).as_bytes(),
            )
            .await?;

            // write headers.
            for (name, value) in response.headers() {
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

            // write body.
            copy(response.body_mut(), self).await?;

            Ok(())
        }
    }
}

impl<T: AsyncWrite + Unpin> ResponseWithStreamBodyWriter for T {}

/// Writer for http request with stream body
pub trait ResponseWriter: AsyncWrite + Unpin {
    /// write request packet into output stream.
    fn write_http_response<T>(
        &mut self,
        response: Response<T>,
    ) -> impl Future<Output = io::Result<()>>
    where
        T: AsRef<[u8]>,
        Self: Sized,
    {
        async move {
            let (parts, body) = response.into_parts();

            let body = Cursor::new(body.as_ref().to_owned());

            ResponseWithStreamBodyWriter::write_http_response(
                self,
                Response::from_parts(parts, body),
            )
            .await
        }
    }
}

impl<T: AsyncWrite + Unpin> ResponseWriter for T {}
#[cfg(test)]
mod tests {

    use futures::io::Cursor;
    use http::{Request, Response, StatusCode};

    use super::*;

    async fn write_request_test(request: Request<&str>, expect: &[u8]) {
        let mut output = Cursor::new(Vec::new());

        RequestWriter::write_http_request(&mut output, request)
            .await
            .unwrap();

        let buf = output.into_inner();

        // use std::str::from_utf8;
        // println!("{}", from_utf8(&buf).unwrap());

        assert_eq!(&buf, expect);
    }

    async fn write_response_test(response: Response<&str>, expect: &[u8]) {
        let mut output = Cursor::new(Vec::new());

        ResponseWriter::write_http_response(&mut output, response)
            .await
            .unwrap();

        let buf = output.into_inner();

        // use std::str::from_utf8;
        // println!("{}", from_utf8(&buf).unwrap());

        assert_eq!(&buf, expect);
    }

    #[futures_test::test]
    async fn test_request() {
        write_request_test(
            Request::get("http://rasi.com").body("").unwrap(),
            b"GET http://rasi.com/ HTTP/1.1\r\n\r\n",
        )
        .await;

        write_request_test(
            Request::get("http://rasi.com").body("hello world").unwrap(),
            b"GET http://rasi.com/ HTTP/1.1\r\n\r\nhello world",
        )
        .await;
    }

    #[futures_test::test]
    async fn test_response() {
        write_response_test(
            Response::builder().status(StatusCode::OK).body("").unwrap(),
            b"HTTP/1.1 200 OK\r\n\r\n",
        )
        .await;

        write_response_test(
            Response::builder()
                .status(StatusCode::OK)
                .body("hello world")
                .unwrap(),
            b"HTTP/1.1 200 OK\r\n\r\nhello world",
        )
        .await;
    }
}
