use std::net::SocketAddr;

use super::tls_config::{
    build_client_connector, build_server_acceptor, parse_url, query_map, required_param,
    server_name, url_host_port,
};
use super::{Listener, Stream};

type DynStream = Box<dyn Stream + Unpin + Send + 'static>;

pub struct TlsListener {
    listener: tokio::net::TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
}

impl TlsListener {
    pub async fn bind(endpoint: &str) -> std::io::Result<Self> {
        let url = parse_url(endpoint, &["tls"])?;
        let params = query_map(&url);
        let cert = required_param(&params, "cert", "XIDL_TLS_CERT")?;
        let key = required_param(&params, "key", "XIDL_TLS_KEY")?;
        let (host, port) = url_host_port(&url)?;
        let bind_host = if host.contains(':') {
            format!("[{host}]")
        } else {
            host
        };
        let listener = tokio::net::TcpListener::bind(format!("{bind_host}:{port}")).await?;
        let acceptor = build_server_acceptor(&cert, &key)?;
        Ok(Self { listener, acceptor })
    }
}

#[async_trait::async_trait]
impl Listener for TlsListener {
    async fn accept(&self) -> std::io::Result<(DynStream, SocketAddr)> {
        loop {
            let (tcp, peer) = self.listener.accept().await?;
            match self.acceptor.accept(tcp).await {
                Ok(tls) => return Ok((Box::new(tls), peer)),
                Err(_) => continue,
            }
        }
    }
}

pub async fn connect_tls(endpoint: &str) -> std::io::Result<DynStream> {
    let url = parse_url(endpoint, &["tls"])?;
    let params = query_map(&url);
    let ca = required_param(&params, "ca", "XIDL_TLS_CA")?;
    let (host, port) = url_host_port(&url)?;
    let server_name = server_name(&host, params.get("server_name").map(|s| s.as_str()))?;
    let connector = build_client_connector(&ca)?;
    let tcp = tokio::net::TcpStream::connect(format!("{host}:{port}")).await?;
    let tls = connector.connect(server_name, tcp).await?;
    Ok(Box::new(tls))
}
