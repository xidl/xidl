use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Mutex, OnceLock};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::{Listener, Stream};

type InprocTx = UnboundedSender<tokio::io::DuplexStream>;

fn registry() -> &'static Mutex<HashMap<String, InprocTx>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, InprocTx>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct InprocListener {
    endpoint: String,
    rx: tokio::sync::Mutex<UnboundedReceiver<tokio::io::DuplexStream>>,
}

impl InprocListener {
    pub fn bind(endpoint: impl Into<String>) -> std::io::Result<Self> {
        let endpoint = endpoint.into();
        let (tx, rx) = unbounded_channel();
        let mut map = registry()
            .lock()
            .map_err(|err| std::io::Error::other(err.to_string()))?;
        if map.contains_key(&endpoint) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("inproc endpoint already in use: {endpoint}"),
            ));
        }
        map.insert(endpoint.clone(), tx);
        Ok(Self {
            endpoint,
            rx: tokio::sync::Mutex::new(rx),
        })
    }
}

impl Drop for InprocListener {
    fn drop(&mut self) {
        if let Ok(mut map) = registry().lock() {
            map.remove(&self.endpoint);
        }
    }
}

pub fn connect_inproc(endpoint: &str) -> std::io::Result<tokio::io::DuplexStream> {
    let map = registry()
        .lock()
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    let tx = map.get(endpoint).cloned().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("inproc endpoint not found: {endpoint}"),
        )
    })?;
    drop(map);

    let (client, server) = tokio::io::duplex(64 * 1024);
    tx.send(server).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            format!("inproc endpoint closed: {endpoint}"),
        )
    })?;
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
        Ok((Box::new(stream), SocketAddr::from(([127, 0, 0, 1], 0))))
    }
}
