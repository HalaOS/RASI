use std::{
    collections::{HashMap, VecDeque},
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use futures::{lock::Mutex, Future, Sink, SinkExt, Stream, StreamExt};

use futures_map::KeyWaitMap;

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

type InnerResponse = Response<String, serde_json::Value, serde_json::Value>;

#[derive(Default)]
struct RawJsonRpcClient {
    is_closed: bool,
    max_send_queue_size: usize,
    send_queue: VecDeque<(usize, Vec<u8>)>,
    received_resps: HashMap<usize, InnerResponse>,
}

impl RawJsonRpcClient {
    fn new(max_send_queue_size: usize) -> Self {
        Self {
            max_send_queue_size,
            ..Default::default()
        }
    }

    fn cache_send(&mut self, id: usize, data: Vec<u8>) -> Option<(usize, Vec<u8>)> {
        if self.send_queue.len() == self.max_send_queue_size {
            return Some((id, data));
        }

        self.send_queue.push_back((id, data));

        None
    }

    fn send_one(&mut self) -> Option<(usize, Vec<u8>)> {
        self.send_queue.pop_front()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum JsonRpcClientEvent {
    Send,
    Forward,
    Response(usize),
}

/// Jsonrpc v2.0 client state machine.
#[derive(Clone)]
pub struct JsonRpcClient {
    next_id: Arc<AtomicUsize>,
    raw: Arc<Mutex<RawJsonRpcClient>>,
    wait_map: Arc<KeyWaitMap<JsonRpcClientEvent, ()>>,
}

impl Default for JsonRpcClient {
    fn default() -> Self {
        Self::new(128)
    }
}

impl JsonRpcClient {
    /// Create a new `JsonRpcClient` with provided send cache channel length.
    pub fn new(max_send_queue_size: usize) -> Self {
        Self {
            next_id: Default::default(),
            raw: Arc::new(Mutex::new(RawJsonRpcClient::new(max_send_queue_size))),
            wait_map: Arc::new(KeyWaitMap::new()),
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

        let mut send_data = Some((id, packet));

        while let Some((id, data)) = send_data {
            let mut raw = self.raw.lock().await;

            Self::verify_client_status(&raw)?;

            send_data = raw.cache_send(id, data);

            if send_data.is_some() {
                self.wait_map.wait(&JsonRpcClientEvent::Send, raw).await;
            } else {
                self.wait_map.insert(JsonRpcClientEvent::Forward, ());
            }
        }

        if let Some(_) = self
            .wait_map
            .wait(&JsonRpcClientEvent::Response(id), ())
            .await
        {
            let mut raw = self.raw.lock().await;

            Self::verify_client_status(&raw)?;

            let resp = raw
                .received_resps
                .remove(&id)
                .expect("consistency guarantee");

            if let Some(err) = resp.error {
                return Err(io::Error::new(io::ErrorKind::Other, err));
            }

            Ok(serde_json::from_value(serde_json::to_value(resp.result)?)?)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "jsonrpc canceled."))
        }
    }

    fn verify_client_status(raw: &RawJsonRpcClient) -> std::io::Result<()> {
        if raw.is_closed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "jsonrpc client is already closed",
            ));
        }

        Ok(())
    }

    /// Writes a single jsonrpc packet to be sent to the peer.
    pub async fn send(&self) -> std::io::Result<(usize, Vec<u8>)> {
        loop {
            let mut raw = self.raw.lock().await;

            Self::verify_client_status(&raw)?;

            if let Some(packet) = raw.send_one() {
                return Ok(packet);
            }

            self.wait_map.insert(JsonRpcClientEvent::Send, ());

            self.wait_map.wait(&JsonRpcClientEvent::Forward, raw).await;
        }
    }

    /// Processes jsonrpc packet received from the peer.
    pub async fn recv<V: AsRef<[u8]>>(&self, packet: V) -> std::io::Result<()> {
        let resp: Response<String, serde_json::Value, serde_json::Value> =
            serde_json::from_slice(packet.as_ref())?;

        let mut raw = self.raw.lock().await;

        let id = resp.id;

        raw.received_resps.insert(resp.id, resp);

        self.wait_map.insert(JsonRpcClientEvent::Response(id), ());

        Ok(())
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
