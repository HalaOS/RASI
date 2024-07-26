use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    lock::Mutex,
    Future, Sink, SinkExt, Stream, StreamExt,
};

use futures_waitmap::WaitMap;

use crate::{Request, Response, Version};

pub trait JsonRpcClientSender<E>: Sink<Vec<u8>, Error = E> + Unpin
where
    E: ToString,
{
    fn send_request<S, P, R, D>(
        &mut self,
        request: Request<S, P>,
    ) -> impl Future<Output = io::Result<()>>
    where
        S: AsRef<str> + serde::Serialize,
        P: serde::Serialize,
    {
        async move {
            let data = serde_json::to_vec(&request)?;

            self.send(data)
                .await
                .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err.to_string()))?;

            Ok(())
        }
    }
}

impl<T, E> JsonRpcClientSender<E> for T
where
    T: Sink<Vec<u8>, Error = E> + Unpin,
    E: ToString,
{
}

pub trait JsonRpcClientReceiver: Stream<Item = Vec<u8>> + Unpin {
    fn next_response<R, D>(&mut self) -> impl Future<Output = io::Result<Response<String, R, D>>>
    where
        for<'a> R: serde::Deserialize<'a>,
        for<'a> D: serde::Deserialize<'a>,
    {
        async move {
            let buf = self.next().await.ok_or(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "JSONRPC client receive stream broken",
            ))?;

            Ok(serde_json::from_slice(&buf)?)
        }
    }
}

impl<T> JsonRpcClientReceiver for T where T: Stream<Item = Vec<u8>> + Unpin {}

/// Jsonrpc v2.0 client state machine.
#[derive(Clone)]
pub struct JsonRpcClient {
    next_id: Arc<AtomicUsize>,
    send_sender: Sender<(usize, Vec<u8>)>,
    send_receiver: Arc<Mutex<Receiver<(usize, Vec<u8>)>>>,
    wait_map: Arc<WaitMap<usize, Response<String, serde_json::Value, serde_json::Value>>>,
}

impl Default for JsonRpcClient {
    fn default() -> Self {
        Self::new(128)
    }
}

impl JsonRpcClient {
    /// Create a new `JsonRpcClient` with provided send cache channel length.
    pub fn new(send_cached_len: usize) -> Self {
        let (send_sender, send_receiver) = channel(send_cached_len);

        Self {
            next_id: Default::default(),
            send_receiver: Arc::new(Mutex::new(send_receiver)),
            send_sender,
            wait_map: Arc::new(WaitMap::new()),
        }
    }

    /// Invoke a jsonrpc v2.0 call and waiting for response.
    pub async fn call<M, P, R>(&mut self, method: M, params: P) -> std::io::Result<R>
    where
        M: AsRef<str>,
        P: serde::Serialize,
        for<'a> R: serde::Deserialize<'a>,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let request = Request {
            id: Some(id),
            jsonrpc: Version::default(),
            method: method.as_ref(),
            params,
        };

        let packet = serde_json::to_vec(&request)?;

        // cache request jsonrpc packet
        self.send_sender
            .send((id, packet))
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;

        if let Some(resp) = self.wait_map.wait(&id).await {
            if let Some(err) = resp.error {
                return Err(io::Error::new(io::ErrorKind::Other, err));
            }

            Ok(serde_json::from_value(serde_json::to_value(resp.result)?)?)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "jsonrpc canceled."))
        }
    }

    /// Writes a single jsonrpc packet to be sent to the peer.
    pub async fn send(&self) -> Option<(usize, Vec<u8>)> {
        self.send_receiver.lock().await.next().await
    }

    /// Processes jsonrpc packet received from the peer.
    pub async fn recv<V: AsRef<[u8]>>(&self, packet: V) -> std::io::Result<()> {
        let resp: Response<String, serde_json::Value, serde_json::Value> =
            serde_json::from_slice(packet.as_ref())?;

        self.wait_map.insert(resp.id, resp).await;

        Ok(())
    }
}

#[cfg(feature = "with_rasi")]

pub mod rasi {
    use std::{any::Any, io, net::ToSocketAddrs, path::Path};

    use futures_http::{
        client::rasio::{HttpClient, HttpClientOptions, HttpClientOptionsBuilder},
        types::{
            request::Builder as RequestBuilder, Error as HttpError, HeaderName, HeaderValue,
            Request, StatusCode, Uri,
        },
    };
    use rasi::task::spawn_ok;
    use serde_json::json;

    use crate::{Error, ErrorCode};

    use super::JsonRpcClient;

    /// A builder to create a http jsonrpc client.
    pub struct HttpJsonRpcClient {
        max_body_size: usize,
        send_cached_len: usize,
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
                max_body_size: 2048,
                send_cached_len: 0,
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

        /// Consume builder and create a new `JsonRpcClient` instance.
        pub fn create(self) -> io::Result<JsonRpcClient> {
            let request = self
                .builder
                .body(())
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

            let (parts, _) = request.into_parts();

            let client = JsonRpcClient::new(self.send_cached_len);

            let background = client.clone();

            let ops: HttpClientOptions = self.send_ops.try_into()?;

            spawn_ok(async move {
                while let Some((id, packet)) = background.send().await {
                    let request = Request::from_parts(parts.clone(), packet);

                    let resp = match request.send(&ops).await {
                        Ok(resp) => resp,
                        Err(err) => {
                            Self::handle_recv(
                                &background,
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
                    };

                    if StatusCode::OK != resp.status() {
                        Self::handle_recv(
                            &background,
                            json!({
                                 "id":id,"jsonrpc":"2.0","error": Error {
                                    code: ErrorCode::InternalError,
                                    message: resp.status().to_string(),
                                    data: None::<()>
                                 }
                            })
                            .to_string(),
                        )
                        .await;

                        continue;
                    }

                    let (_, body) = resp.into_parts();

                    let packet = match body.into_bytes(self.max_body_size).await {
                        Err(err) => {
                            Self::handle_recv(
                                &background,
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
                        Ok(buf) => buf,
                    };

                    Self::handle_recv(&background, packet).await;
                }
            });

            Ok(client)
        }

        async fn handle_recv<P: AsRef<[u8]>>(client: &JsonRpcClient, packet: P) {
            if let Err(err) = client.recv(packet).await {
                log::error!("handle http jsonrpc recv with error:: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::task::Poll;

    use futures::poll;
    use serde_json::json;

    use crate::{Error, ErrorCode};

    use super::*;

    #[futures_test::test]
    async fn test_empty_return() {
        let client = JsonRpcClient::default();

        let mut call_client = client.clone();

        let mut call = Box::pin(call_client.call("echo", ("hello", 1)));

        let poll_result: Poll<io::Result<()>> = poll!(&mut call);

        assert!(poll_result.is_pending());

        let (_, buf) = client.send().await.unwrap();

        let json = json!({"id":0,"jsonrpc":"2.0","method":"echo","params":["hello",1]}).to_string();

        assert_eq!(json.as_bytes(), buf);

        client
            .recv(
                json!({
                    "id":0,"jsonrpc":"2.0"
                })
                .to_string(),
            )
            .await
            .unwrap();

        let poll_result: Poll<io::Result<()>> = poll!(&mut call);

        assert!(matches!(poll_result, Poll::Ready(Ok(()))));

        let mut call_client = client.clone();

        let mut call = Box::pin(call_client.call("echo", ("hello", 1)));

        let poll_result: Poll<io::Result<i32>> = poll!(&mut call);

        assert!(poll_result.is_pending());

        client
            .recv(
                json!({
                    "id":1,"jsonrpc":"2.0","result":1
                })
                .to_string(),
            )
            .await
            .unwrap();

        let poll_result = poll!(&mut call);

        assert!(matches!(poll_result, Poll::Ready(Ok(1))));

        let mut call_client = client.clone();

        let mut call = Box::pin(call_client.call("echo", ("hello", 1)));

        let poll_result: Poll<io::Result<i32>> = poll!(&mut call);

        assert!(poll_result.is_pending());

        client
            .recv(
                json!({
                    "id":2,"jsonrpc":"2.0","error": Error {
                        code: ErrorCode::InternalError,
                        message: "",
                        data: None::<()>
                    }
                })
                .to_string(),
            )
            .await
            .unwrap();

        let poll_result = poll!(&mut call);

        assert!(matches!(poll_result, Poll::Ready(Err(_))));
    }
}
