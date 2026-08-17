#[cfg(test)]
mod test;

use crate::codec::Codec;
use crate::transport::Stream;
use crate::{Error, JSONRPC_VERSION, RpcRequest, RpcResponse};
use dashmap::DashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufRead, BufReader, BufStream, BufWriter, WriteHalf};
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;

/// A simple JSON-RPC client that preserves the caller-owned stream lifetime.
pub struct Client<S> {
    stream: BufStream<S>,
    next_id: u64,
    codec: Codec,
}

impl<S> Client<S>
where
    S: Stream + Unpin,
{
    /// Creates a client using newline-delimited JSON framing.
    pub fn new(stream: S) -> Self {
        Self::with_codec(stream, Codec::Json)
    }

    /// Creates a client using length-prefixed MessagePack framing.
    #[cfg(feature = "msgpack")]
    pub fn new_msgpack(stream: S) -> Self {
        Self::with_codec(stream, Codec::Msgpack)
    }

    fn with_codec(stream: S, codec: Codec) -> Self {
        Self {
            stream: BufStream::new(stream),
            next_id: 1,
            codec,
        }
    }

    /// Sends a request and awaits its matching response.
    pub async fn call<P, T>(&mut self, method: &str, params: P) -> Result<T, Error>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let id = self.next_id;
        self.next_id += 1;

        let request = RpcRequest {
            jsonrpc: JSONRPC_VERSION,
            id,
            method,
            params,
        };
        self.codec.write(&mut self.stream, &request).await?;

        loop {
            let Some(value) = self.codec.read::<_, Value>(&mut self.stream).await? else {
                return Err(Error::Protocol("no response"));
            };
            if is_notification(&value) {
                continue;
            }
            let response: RpcResponse = serde_json::from_value(value)?;
            if response.id.as_ref() != Some(&Value::from(id)) {
                return Err(Error::Protocol("unexpected JSON-RPC id"));
            }
            response.validate()?;
            return Ok(serde_json::from_value(response.into_result()?)?);
        }
    }
}

/// A concurrent client for an owned stream.
///
/// Unlike [`Client`], this type dispatches responses in a background task, so
/// generated clients can share one connection without an outer request mutex.
pub struct ConcurrentClient<S> {
    writer: Arc<Mutex<BufWriter<WriteHalf<S>>>>,
    pending: Arc<Pending>,
    next_id: AtomicU64,
    codec: Codec,
    closed: Arc<AtomicBool>,
    read_task: JoinHandle<()>,
    _stream: PhantomData<fn() -> S>,
}

type Pending = DashMap<u64, oneshot::Sender<Result<Value, Error>>>;

impl<S> ConcurrentClient<S>
where
    S: Stream + Unpin + Send + 'static,
{
    /// Creates a concurrent JSON client.
    pub fn new(stream: S) -> Self {
        Self::with_codec(stream, Codec::Json)
    }

    /// Creates a concurrent MessagePack client.
    #[cfg(feature = "msgpack")]
    pub fn new_msgpack(stream: S) -> Self {
        Self::with_codec(stream, Codec::Msgpack)
    }

    fn with_codec(stream: S, codec: Codec) -> Self {
        let (read_half, write_half) = tokio::io::split(stream);
        let pending = Arc::new(Pending::new());
        let closed = Arc::new(AtomicBool::new(false));
        let read_task = Self::spawn_reader(
            BufReader::new(read_half),
            codec,
            pending.clone(),
            closed.clone(),
        );
        Self {
            writer: Arc::new(Mutex::new(BufWriter::new(write_half))),
            pending,
            next_id: AtomicU64::new(1),
            codec,
            closed,
            read_task,
            _stream: PhantomData,
        }
    }

    /// Sends a request and awaits its matching response.
    pub async fn call<P, T>(&self, method: &str, params: P) -> Result<T, Error>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::Protocol("rpc client closed"));
        }
        let request_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.pending.insert(request_id, sender);
        let _guard = PendingRequest {
            id: request_id,
            pending: self.pending.clone(),
        };
        let request = RpcRequest {
            jsonrpc: JSONRPC_VERSION,
            id: request_id,
            method,
            params,
        };
        if let Err(error) = self
            .codec
            .write(&mut *self.writer.lock().await, &request)
            .await
        {
            self.closed.store(true, Ordering::Release);
            fail_pending(&self.pending, "rpc client write failed");
            return Err(error);
        }
        match tokio::time::timeout(Duration::from_secs(30), receiver).await {
            Ok(Ok(Ok(value))) => Ok(serde_json::from_value(value)?),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err(Error::Protocol("rpc request canceled")),
            Err(_) => Err(Error::RequestTimeout),
        }
    }

    /// Sends a fire-and-forget notification.
    pub async fn notify<P>(&self, method: &str, params: P) -> Result<(), Error>
    where
        P: Serialize,
    {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::Protocol("rpc client closed"));
        }
        let request = serde_json::json!({
            "jsonrpc": JSONRPC_VERSION,
            "method": method,
            "params": params,
        });
        let result = self
            .codec
            .write(&mut *self.writer.lock().await, &request)
            .await;
        if result.is_err() {
            self.closed.store(true, Ordering::Release);
            fail_pending(&self.pending, "rpc client write failed");
        }
        result
    }

    fn spawn_reader<R>(
        mut reader: R,
        codec: Codec,
        pending: Arc<Pending>,
        closed: Arc<AtomicBool>,
    ) -> JoinHandle<()>
    where
        R: AsyncBufRead + Unpin + Send + 'static,
    {
        tokio::spawn(async move {
            loop {
                let value = match codec.read::<_, Value>(&mut reader).await {
                    Ok(Some(value)) => value,
                    Ok(None) | Err(_) => break,
                };
                route_value(value, &pending);
            }
            closed.store(true, Ordering::Release);
            fail_pending(&pending, "rpc stream closed");
        })
    }
}

impl<S> Drop for ConcurrentClient<S> {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);
        fail_pending(&self.pending, "rpc client closed");
        self.read_task.abort();
    }
}

struct PendingRequest {
    id: u64,
    pending: Arc<Pending>,
}

impl Drop for PendingRequest {
    fn drop(&mut self) {
        self.pending.remove(&self.id);
    }
}

fn is_notification(value: &Value) -> bool {
    value.get("id").is_none() && value.get("method").is_some()
}

fn route_value(value: Value, pending: &Pending) {
    if let Value::Array(values) = value {
        for value in values {
            route_value(value, pending);
        }
        return;
    }
    let Some(id) = value.get("id").and_then(Value::as_u64) else {
        return;
    };
    let Some((_, sender)) = pending.remove(&id) else {
        return;
    };
    let result = serde_json::from_value::<RpcResponse>(value)
        .map_err(Error::from)
        .and_then(|response| response.into_result());
    let _ = sender.send(result);
}

fn fail_pending(pending: &Pending, message: &'static str) {
    let ids = pending.iter().map(|entry| *entry.key()).collect::<Vec<_>>();
    for id in ids {
        if let Some((_, sender)) = pending.remove(&id) {
            let _ = sender.send(Err(Error::Protocol(message)));
        }
    }
}
