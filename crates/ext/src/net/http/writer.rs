use std::io;

use http::{header::ToStrError, Request, Response};
use rasi::io::{copy, AsyncRead, AsyncWrite, AsyncWriteExt, Cursor};

fn map_to_str_error(err: ToStrError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

/// A http response writer, that write response packet into output stream.
pub struct RequestWriter<'a, S> {
    output: &'a mut S,
}

impl<'a, S> RequestWriter<'a, S> {
    /// Create new `ResponseWriter` with `output` stream.
    pub fn new(output: &'a mut S) -> Self {
        Self { output }
    }

    /// write request packet into output stream.
    pub async fn write_with_stream_body<T>(mut self, mut request: Request<T>) -> io::Result<()>
    where
        S: AsyncWrite + Unpin,
        T: AsyncRead + Unpin,
    {
        // write status line.
        self.output
            .write_all(
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
            self.output
                .write_all(
                    format!(
                        "{}: {}\r\n",
                        name,
                        value.to_str().map_err(map_to_str_error)?
                    )
                    .as_bytes(),
                )
                .await?;
        }

        self.output.write_all(b"\r\n").await?;

        // write body.
        copy(request.body_mut(), &mut self.output).await?;

        Ok(())
    }

    /// Write
    pub async fn write<T>(self, request: Request<T>) -> io::Result<()>
    where
        S: AsyncWrite + Unpin,
        T: AsRef<[u8]>,
    {
        let (parts, body) = request.into_parts();

        let request = Request::from_parts(parts, Cursor::new(body.as_ref()));

        self.write_with_stream_body(request).await
    }
}

/// A http response writer, that write response packet into output stream.
pub struct ResponseWriter<S> {
    output: S,
}

impl<S> ResponseWriter<S> {
    /// Create new `ResponseWriter` with `output` stream.
    pub fn new(output: S) -> Self {
        Self { output }
    }

    /// write response packet which body type is a [`AsyncRead`]
    pub async fn write_with_stream_body<T>(mut self, mut response: Response<T>) -> io::Result<()>
    where
        S: AsyncWrite + Unpin,
        T: AsyncRead + Unpin,
    {
        // write status line.
        self.output
            .write_all(format!("{:?} {}\r\n", response.version(), response.status()).as_bytes())
            .await?;

        // write headers.
        for (name, value) in response.headers() {
            self.output
                .write_all(
                    format!(
                        "{}: {}\r\n",
                        name,
                        value.to_str().map_err(map_to_str_error)?
                    )
                    .as_bytes(),
                )
                .await?;
        }

        self.output.write_all(b"\r\n").await?;

        // write body.
        copy(response.body_mut(), &mut self.output).await?;

        Ok(())
    }

    pub async fn write<T>(self, response: Response<T>) -> io::Result<()>
    where
        S: AsyncWrite + Unpin,
        T: AsRef<[u8]>,
    {
        let (parts, body) = response.into_parts();

        let response = Response::from_parts(parts, Cursor::new(body.as_ref()));

        self.write_with_stream_body(response).await
    }
}

#[cfg(test)]
mod tests {

    use http::{Request, Response, StatusCode};
    use rasi::io::Cursor;

    use super::*;

    async fn write_request_test(request: Request<&str>, expect: &[u8]) {
        let mut output = Cursor::new(Vec::new());

        RequestWriter::new(&mut output)
            .write(request)
            .await
            .unwrap();

        let buf = output.into_inner();

        // use std::str::from_utf8;
        // println!("{}", from_utf8(&buf).unwrap());

        assert_eq!(&buf, expect);
    }

    async fn write_response_test(response: Response<&str>, expect: &[u8]) {
        let mut output = Cursor::new(Vec::new());

        ResponseWriter::new(&mut output)
            .write(response)
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
