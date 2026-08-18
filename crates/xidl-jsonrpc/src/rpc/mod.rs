//! Concurrent request/response correlation over a bidirectional stream.
//!
//! [`RpcClient`] multiplexes JSON-RPC requests over one [`ReaderWriter`]:
//! every `call` allocates a unique id, a background dispatch loop matches
//! incoming responses back to their pending request by id, and notifications
//! pushed by the peer without an id are delivered through an unbounded
//! channel. Request ids start after the id reserved by the stream handshake.

use crate::stream::{ClientStreamWriter, Reader, ReaderWriter};
use crate::{Error, ErrorCode, JSONRPC_VERSION};
use dashmap::DashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;

/// Default time a request waits for its response before timing out.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Pending request registrations keyed by request id.
type Pending = DashMap<u64, oneshot::Sender<Result<Value, Error>>>;

/// A concurrent JSON-RPC client multiplexing requests over one stream.
///
/// Requests and notifications share the underlying bidirectional stream. Each
/// in-flight request gets a unique id and its response is matched back by id;
/// server pushes without an id are delivered through the notification receiver
/// returned by [`RpcClient::new`]. After the stream closes or the client is
/// dropped, every pending request fails immediately instead of waiting for its
/// timeout.
pub struct RpcClient {
    writer: Mutex<ClientStreamWriter<Value, ()>>,
    pending: Arc<Pending>,
    next_request_id: AtomicU64,
    request_timeout: Duration,
    read_task: JoinHandle<()>,
    closed: Arc<AtomicBool>,
}

impl RpcClient {
    /// Creates a concurrent client over an established bidirectional stream.
    ///
    /// Returns the client and the receiver for server-pushed notifications.
    /// Requests that do not get a response within 30 seconds fail with
    /// [`Error::RequestTimeout`].
    pub fn new(session: ReaderWriter<Value, Value>) -> (Self, mpsc::UnboundedReceiver<Value>) {
        Self::with_timeout(session, DEFAULT_REQUEST_TIMEOUT)
    }

    /// Creates a concurrent client with a custom per-request timeout.
    pub fn with_timeout(
        session: ReaderWriter<Value, Value>,
        request_timeout: Duration,
    ) -> (Self, mpsc::UnboundedReceiver<Value>) {
        let (writer, reader) = session.into_parts();
        let pending = Arc::new(Pending::new());
        let closed = Arc::new(AtomicBool::new(false));
        let (notifications_tx, notifications_rx) = mpsc::unbounded_channel();
        let read_task =
            Self::spawn_dispatch(reader, pending.clone(), notifications_tx, closed.clone());
        let next_request_id = AtomicU64::new(2);
        (
            Self {
                writer: Mutex::new(writer),
                pending,
                next_request_id,
                request_timeout,
                read_task,
                closed,
            },
            notifications_rx,
        )
    }

    /// Sends a JSON-RPC request and awaits its correlated response.
    ///
    /// The future stays pending until the peer answers with a matching id, the
    /// per-request timeout elapses, or the stream closes and fails the request.
    pub async fn call<P, T>(&self, method: &str, params: P) -> Result<T, Error>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Protocol("rpc client closed"));
        }
        let params = serde_json::to_value(params)?;
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.insert(request_id, tx);
        let _guard = PendingRequest {
            request_id,
            pending: self.pending.clone(),
        };

        let request = json!({
            "jsonrpc": JSONRPC_VERSION,
            "id": request_id,
            "method": method,
            "params": params,
        });
        if let Err(error) = self.writer.lock().await.write(request).await {
            self.closed.store(true, Ordering::SeqCst);
            Self::fail_all(&self.pending, "rpc stream write failed");
            return Err(error);
        }

        match tokio::time::timeout(self.request_timeout, rx).await {
            Ok(Ok(Ok(value))) => serde_json::from_value(value).map_err(Error::from),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err(Error::Protocol("rpc request canceled")),
            Err(_) => Err(Error::RequestTimeout),
        }
    }

    /// Sends a fire-and-forget JSON-RPC notification.
    pub async fn notify<P>(&self, method: &str, params: P) -> Result<(), Error>
    where
        P: Serialize,
    {
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Protocol("rpc client closed"));
        }
        let params = serde_json::to_value(params)?;
        let message = json!({
            "jsonrpc": JSONRPC_VERSION,
            "method": method,
            "params": params,
        });
        self.writer.lock().await.write(message).await
    }
}

impl Drop for RpcClient {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
        Self::fail_all(&self.pending, "rpc client closed");
        self.read_task.abort();
    }
}

/// Removes `request_id` from the pending map when a caller drops its `call`.
struct PendingRequest {
    request_id: u64,
    pending: Arc<Pending>,
}

impl Drop for PendingRequest {
    fn drop(&mut self) {
        self.pending.remove(&self.request_id);
    }
}

impl RpcClient {
    /// Spawns the background task that routes incoming messages by request id.
    fn spawn_dispatch(
        mut reader: Reader<'static, Value>,
        pending: Arc<Pending>,
        notifications: mpsc::UnboundedSender<Value>,
        closed: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(result) = reader.read().await {
                match result {
                    Ok(value) => Self::route_message(value, &pending, &notifications),
                    Err(_) => break,
                }
            }
            closed.store(true, Ordering::SeqCst);
            Self::fail_all(&pending, "rpc stream closed");
        })
    }

    /// Routes one incoming message to its pending request or the notifications.
    fn route_message(
        value: Value,
        pending: &Pending,
        notifications: &mpsc::UnboundedSender<Value>,
    ) {
        if let Value::Array(items) = value {
            for item in items {
                Self::route_message(item, pending, notifications);
            }
            return;
        }
        if value.get("id").is_some() {
            if let Some(request_id) = value.get("id").and_then(Value::as_u64) {
                if let Some((_, tx)) = pending.remove(&request_id) {
                    let _ = tx.send(Self::classify_response(value));
                }
            }
            return;
        }
        let _ = notifications.send(value);
    }

    /// Turns a JSON-RPC response value into its result or error.
    fn classify_response(response: Value) -> Result<Value, Error> {
        match response.get("error") {
            Some(error) => {
                let code = error
                    .get("code")
                    .and_then(Value::as_i64)
                    .unwrap_or(ErrorCode::ServerError.code());
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("rpc response error")
                    .to_owned();
                Err(Error::Rpc {
                    code: ErrorCode::from(code),
                    message,
                    data: error.get("data").cloned(),
                })
            }
            None => Ok(response.get("result").cloned().unwrap_or(Value::Null)),
        }
    }

    /// Fails every pending request because the underlying stream is gone.
    fn fail_all(pending: &Pending, message: &'static str) {
        let request_ids: Vec<u64> = pending.iter().map(|entry| *entry.key()).collect();
        for request_id in request_ids {
            if let Some((_, tx)) = pending.remove(&request_id) {
                let _ = tx.send(Err(Error::Protocol(message)));
            }
        }
    }
}

#[cfg(test)]
mod tests;
