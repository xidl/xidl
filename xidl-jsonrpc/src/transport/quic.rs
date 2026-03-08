use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use s2n_quic::client::Connect;
use tokio::sync::{Mutex, mpsc};

use super::{Listener, Stream};

type DynStream = Box<dyn Stream + Unpin + Send + 'static>;

const DEFAULT_CERT: &str = "cert.pem";
const DEFAULT_KEY: &str = "key.pem";
const DEFAULT_CA: &str = "cert.pem";
const DEFAULT_SERVER_NAME: &str = "localhost";

struct QuicEndpoint {
    addr: String,
    params: HashMap<String, String>,
}

impl QuicEndpoint {
    fn parse(endpoint: &str) -> std::io::Result<Self> {
        let addr = endpoint
            .strip_prefix("quic://")
            .unwrap_or(endpoint)
            .to_string();
        let mut parts = addr.splitn(2, '?');
        let host = parts.next().unwrap_or_default().to_string();
        if host.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "missing quic address",
            ));
        }
        let mut params = HashMap::new();
        if let Some(query) = parts.next() {
            for pair in query.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let mut kv = pair.splitn(2, '=');
                let key = kv.next().unwrap_or_default();
                if key.is_empty() {
                    continue;
                }
                let value = kv.next().unwrap_or_default();
                params.insert(key.to_string(), value.to_string());
            }
        }
        Ok(Self { addr: host, params })
    }

    fn cert_path(&self) -> String {
        self.params
            .get("cert")
            .cloned()
            .or_else(|| std::env::var("XIDL_QUIC_CERT").ok())
            .unwrap_or_else(|| DEFAULT_CERT.to_string())
    }

    fn key_path(&self) -> String {
        self.params
            .get("key")
            .cloned()
            .or_else(|| std::env::var("XIDL_QUIC_KEY").ok())
            .unwrap_or_else(|| DEFAULT_KEY.to_string())
    }

    fn ca_path(&self) -> String {
        self.params
            .get("ca")
            .cloned()
            .or_else(|| std::env::var("XIDL_QUIC_CA").ok())
            .unwrap_or_else(|| DEFAULT_CA.to_string())
    }

    fn server_name(&self) -> String {
        self.params
            .get("server_name")
            .cloned()
            .or_else(|| std::env::var("XIDL_QUIC_SERVER_NAME").ok())
            .unwrap_or_else(|| DEFAULT_SERVER_NAME.to_string())
    }
}

fn io_other<E: std::fmt::Display>(err: E) -> std::io::Error {
    std::io::Error::other(err.to_string())
}

pub struct QuicListener {
    rx: Mutex<mpsc::UnboundedReceiver<(DynStream, SocketAddr)>>,
    _accept_task: tokio::task::JoinHandle<()>,
}

impl QuicListener {
    pub fn bind(endpoint: &str) -> std::io::Result<Self> {
        let cfg = QuicEndpoint::parse(endpoint)?;
        let cert = cfg.cert_path();
        let key = cfg.key_path();
        let mut server = s2n_quic::Server::builder()
            .with_tls((cert.as_str(), key.as_str()))
            .map_err(io_other)?
            .with_io(cfg.addr.as_str())
            .map_err(io_other)?
            .start()
            .map_err(io_other)?;

        let (tx, rx) = mpsc::unbounded_channel::<(DynStream, SocketAddr)>();
        let task = tokio::spawn(async move {
            while let Some(mut connection) = server.accept().await {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let peer = connection
                        .remote_addr()
                        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 0)));
                    while let Ok(Some(stream)) = connection.accept_bidirectional_stream().await {
                        if tx.send((Box::new(stream), peer)).is_err() {
                            break;
                        }
                    }
                });
            }
        });
        Ok(Self {
            rx: Mutex::new(rx),
            _accept_task: task,
        })
    }
}

#[async_trait::async_trait]
impl Listener for QuicListener {
    async fn accept(&self) -> std::io::Result<(DynStream, SocketAddr)> {
        let mut rx = self.rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "quic listener closed")
        })
    }
}

struct QuicClientConnection {
    _client: s2n_quic::Client,
    connection: Mutex<s2n_quic::connection::Connection>,
}

type ConnectionCache = DashMap<String, Arc<QuicClientConnection>>;
static CONNECTIONS: LazyLock<ConnectionCache> = LazyLock::new(DashMap::new);

pub async fn connect_quic(endpoint: &str) -> std::io::Result<DynStream> {
    let key = endpoint.to_string();
    if let Some(entry) = CONNECTIONS.get(&key) {
        let mut connection = entry.connection.lock().await;
        let stream = connection
            .open_bidirectional_stream()
            .await
            .map_err(io_other)?;
        return Ok(Box::new(stream));
    }

    let cfg = QuicEndpoint::parse(endpoint)?;
    let client = s2n_quic::Client::builder()
        .with_tls(cfg.ca_path().as_str())
        .map_err(io_other)?
        .with_io(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        .map_err(io_other)?
        .start()
        .map_err(io_other)?;
    let connect = Connect::new(cfg.addr.parse::<SocketAddr>().map_err(io_other)?)
        .with_server_name(cfg.server_name());
    let connection = client.connect(connect).await.map_err(io_other)?;

    let shared = Arc::new(QuicClientConnection {
        _client: client,
        connection: Mutex::new(connection),
    });
    let mut guard = shared.connection.lock().await;
    let stream = guard.open_bidirectional_stream().await.map_err(io_other)?;
    drop(guard);
    CONNECTIONS.insert(key, shared);
    Ok(Box::new(stream))
}
