use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use dashmap::DashMap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::{Listener, Stream, loopback_peer_addr};

type InprocStream = tokio::io::DuplexStream;

struct BoundSlot {
    listener_id: u64,
    tx: UnboundedSender<InprocStream>,
}

#[derive(Default)]
struct EndpointEntry {
    bound: Option<BoundSlot>,
    pending: VecDeque<InprocStream>,
}

fn next_listener_id() -> u64 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

type EndpointState = Arc<Mutex<EndpointEntry>>;
type Registry = DashMap<String, EndpointState>;
static REGISTRY: LazyLock<Registry> = LazyLock::new(DashMap::new);

pub struct InprocListener {
    listener_id: u64,
    endpoint: String,
    rx: tokio::sync::Mutex<UnboundedReceiver<InprocStream>>,
}

impl InprocListener {
    pub fn bind(endpoint: impl Into<String>) -> std::io::Result<Self> {
        let endpoint = endpoint.into();
        let listener_id = next_listener_id();
        let (tx, rx) = unbounded_channel();

        let entry = REGISTRY
            .entry(endpoint.clone())
            .or_insert_with(|| Arc::new(Mutex::new(EndpointEntry::default())))
            .clone();

        let mut guard = entry
            .lock()
            .map_err(|err| std::io::Error::other(err.to_string()))?;
        if guard.bound.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("inproc endpoint already in use: {endpoint}"),
            ));
        }

        guard.bound = Some(BoundSlot {
            listener_id,
            tx: tx.clone(),
        });

        while let Some(stream) = guard.pending.pop_front() {
            if let Err(err) = tx.send(stream) {
                let failed = err.0;
                guard.bound = None;
                guard.pending.push_front(failed);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("inproc listener channel closed for endpoint: {endpoint}"),
                ));
            }
        }
        drop(guard);

        Ok(Self {
            listener_id,
            endpoint,
            rx: tokio::sync::Mutex::new(rx),
        })
    }
}

impl Drop for InprocListener {
    fn drop(&mut self) {
        if let Some(state) = REGISTRY.get(&self.endpoint).map(|entry| entry.clone()) {
            let mut should_remove = false;
            if let Ok(mut entry) = state.lock() {
                if entry
                    .bound
                    .as_ref()
                    .map(|slot| slot.listener_id == self.listener_id)
                    .unwrap_or(false)
                {
                    entry.bound = None;
                }
                should_remove = entry.bound.is_none() && entry.pending.is_empty();
            }
            if should_remove {
                REGISTRY.remove(&self.endpoint);
            }
        }
    }
}

pub fn connect_inproc(endpoint: &str) -> std::io::Result<InprocStream> {
    let (client, server) = tokio::io::duplex(64 * 1024);
    let state = REGISTRY
        .entry(endpoint.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(EndpointEntry::default())))
        .clone();
    let mut entry = state
        .lock()
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    if let Some(bound) = entry.bound.as_ref() {
        if let Err(err) = bound.tx.send(server) {
            let failed = err.0;
            entry.bound = None;
            entry.pending.push_back(failed);
        }
    } else {
        entry.pending.push_back(server);
    }
    Ok(client)
}

#[async_trait::async_trait]
impl Listener for InprocListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn Stream + Unpin + Send + 'static>, SocketAddr)> {
        let mut rx = self.rx.lock().await;
        let stream = rx.recv().await.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "inproc listener closed")
        })?;
        Ok((Box::new(stream), loopback_peer_addr()))
    }

    fn endpoint(&self) -> Option<String> {
        Some(format!("inproc://{}", self.endpoint))
    }
}
