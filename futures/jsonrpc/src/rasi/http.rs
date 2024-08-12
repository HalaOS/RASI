use std::{any::Any, io, net::ToSocketAddrs, path::Path, str::from_utf8, time::Duration};

use futures::TryStreamExt;
use futures_http::{
    body::BodyReader,
    client::rasio::{HttpClient, HttpClientOptions, HttpClientOptionsBuilder},
    types::{
        request::{Builder as RequestBuilder, Parts},
        Error as HttpError, HeaderName, HeaderValue, Request, StatusCode, Uri,
    },
};
use rasi::{task::spawn_ok, timer::TimeoutExt};
use serde_json::json;

use crate::{client::JsonRpcClient, Error, ErrorCode};

/// A builder to create a http jsonrpc client.
pub struct HttpJsonRpcClient {
    max_body_size: usize,
    send_cached_len: usize,
    timeout: Duration,
    builder: RequestBuilder,
    send_ops: HttpClientOptionsBuilder,
}

impl HttpJsonRpcClient {
    /// Create new http rpc client builder with server uri.
    pub fn new<T>(uri: T) -> Self
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<HttpError>,
    {
        HttpJsonRpcClient {
            max_body_size: 1024 * 1024,
            send_cached_len: 10,
            timeout: Duration::from_secs(5),
            builder: RequestBuilder::new().method("POST").uri(uri),
            send_ops: HttpClientOptions::new(),
        }
    }

    /// Appends a http header to the http `JsonRpcClient` builder.
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<HttpError>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<HttpError>,
    {
        self.builder = self.builder.header(key, value);

        self
    }

    /// Add an extension to the http `JsonRpcClient` builder.
    pub fn extension<T>(mut self, extension: T) -> Self
    where
        T: Clone + Any + Send + Sync + 'static,
    {
        self.builder = self.builder.extension(extension);
        self
    }

    /// Rewrite http request's host:port fields and send request to the specified `raddrs`.
    pub fn redirect<R: ToSocketAddrs>(mut self, raddrs: R) -> Self {
        self.send_ops = self.send_ops.redirect(raddrs);

        self
    }

    /// Set remote server's server name, this option will rewrite request's host field.
    pub fn with_server_name(mut self, server_name: &str) -> Self {
        self.send_ops = self.send_ops.with_server_name(server_name);

        self
    }

    /// Set the server verification ca file, this is useful for self signed server.
    pub fn with_ca_file<P: AsRef<Path>>(mut self, ca_file: P) -> Self {
        self.send_ops = self.send_ops.with_ca_file(ca_file);
        self
    }

    /// Set the timeout duration of the jsonrpc call via http request.
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Configures the use of Server Name Indication (SNI) when connecting.
    /// Defaults to true.
    pub fn set_use_server_name_indication(mut self, value: bool) -> Self {
        self.send_ops = self.send_ops.set_use_server_name_indication(value);
        self
    }

    /// Consume builder and create a new `JsonRpcClient` instance.
    pub fn create(self) -> io::Result<JsonRpcClient> {
        let client = JsonRpcClient::new(self.send_cached_len);

        let background = client.clone();

        spawn_ok(async move {
            if let Err(err) = self.run_loop(background).await {
                log::error!(target: "HttpJsonRpcClient", "stop background task, {}",err);
            } else {
                log::info!(target: "HttpJsonRpcClient", "stop background task");
            }
        });

        Ok(client)
    }

    async fn run_loop(self, background: JsonRpcClient) -> std::io::Result<()> {
        let request = self
            .builder
            .body(())
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

        let (parts, _) = request.into_parts();

        let ops: HttpClientOptions = self.send_ops.try_into()?;

        loop {
            let (id, packet) = background.send().await?;

            log::trace!("send jsonrpc: {}", from_utf8(&packet).unwrap());

            let call = Self::send_request(&ops, self.max_body_size, parts.clone(), packet)
                .timeout(self.timeout)
                .await;

            let buf = match call {
                Some(Ok(buf)) => buf,
                Some(Err(err)) => {
                    _ = background
                        .recv(
                            json!({
                            "id":id,"jsonrpc":"2.0","error": Error {
                                         code: ErrorCode::InternalError,
                                         message: err.to_string(),
                                         data: None::<()>
                                      }
                             })
                            .to_string(),
                        )
                        .await;

                    continue;
                }
                None => {
                    _ = background
                        .recv(
                            json!({
                            "id":id,"jsonrpc":"2.0","error": Error {
                                         code: ErrorCode::InternalError,
                                         message: "Timeout",
                                         data: None::<()>
                                      }
                             })
                            .to_string(),
                        )
                        .await;

                    continue;
                }
            };

            Self::handle_recv(&background, buf).await;
        }
    }

    async fn handle_recv<P: AsRef<[u8]>>(client: &JsonRpcClient, packet: P) {
        log::trace!("recv jsonrpc: {}", from_utf8(packet.as_ref()).unwrap());

        if let Err(err) = client.recv(packet).await {
            log::error!("handle http jsonrpc recv with error: {}", err);
        }
    }

    async fn send_request(
        ops: &HttpClientOptions,
        max_body_size: usize,
        parts: Parts,
        packet: Vec<u8>,
    ) -> io::Result<Vec<u8>> {
        let request = Request::from_parts(parts, BodyReader::from(packet));

        let resp = request.send(ops).await?;

        if StatusCode::OK != resp.status() {
            return Err(io::Error::new(io::ErrorKind::Other, resp.status().as_str()));
        }

        let (_, mut body) = resp.into_parts();

        let mut buf = vec![];

        while let Some(mut chunk) = body.try_next().await? {
            buf.append(&mut chunk);

            if buf.len() > max_body_size {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Body length too long: {}", max_body_size),
                ));
            }
        }

        Ok(buf)
    }
}
