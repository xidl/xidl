use std::net::SocketAddr;

use super::tls_config::{TransportUrl, build_client_connector, build_server_acceptor, server_name};
use super::{Listener, Stream};

type DynStream = Box<dyn Stream + Unpin + Send + 'static>;

pub struct TlsListener {
    listener: tokio::net::TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
}

impl TlsListener {
    pub async fn bind(endpoint: &str) -> std::io::Result<Self> {
        let endpoint = TransportUrl::parse(endpoint, &["tls"])?;
        let cert = endpoint.required_param("cert", "XIDL_TLS_CERT")?;
        let key = endpoint.required_param("key", "XIDL_TLS_KEY")?;
        let listener = tokio::net::TcpListener::bind(endpoint.bind_addr()?).await?;
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
    let endpoint = TransportUrl::parse(endpoint, &["tls"])?;
    let ca = endpoint.required_param("ca", "XIDL_TLS_CA")?;
    let (host, port) = endpoint.host_port()?;
    let server_name = server_name(&host, endpoint.optional_param("server_name"))?;
    let connector = build_client_connector(&ca)?;
    let tcp = tokio::net::TcpStream::connect(format!("{host}:{port}")).await?;
    let tls = connector.connect(server_name, tcp).await?;
    Ok(Box::new(tls))
}
