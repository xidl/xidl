use std::collections::HashMap;
#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
use std::fs::File;
#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
use std::io::BufReader;
#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
use std::sync::Arc;

#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
use rustls::{ClientConfig, RootCertStore, ServerConfig};
#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
use tokio_rustls::{TlsAcceptor, TlsConnector};
use url::Url;

pub(crate) fn io_other<E: std::fmt::Display>(err: E) -> std::io::Error {
    std::io::Error::other(err.to_string())
}

#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
pub(crate) fn socket_bind_addr(host: &str, port: u16) -> String {
    let bind_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    format!("{bind_host}:{port}")
}

pub(crate) struct TransportUrl {
    url: Url,
    params: HashMap<String, String>,
}

impl TransportUrl {
    pub(crate) fn parse(endpoint: &str, expected_schemes: &[&str]) -> std::io::Result<Self> {
        let url = Url::parse(endpoint).map_err(io_other)?;
        if !expected_schemes
            .iter()
            .any(|scheme| *scheme == url.scheme())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unsupported scheme: {}", url.scheme()),
            ));
        }
        let params = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Ok(Self { url, params })
    }

    #[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
    pub(crate) fn scheme(&self) -> &str {
        self.url.scheme()
    }

    #[cfg(feature = "tokio-websocket")]
    pub(crate) fn as_str(&self) -> &str {
        self.url.as_str()
    }

    pub(crate) fn host_port(&self) -> std::io::Result<(String, u16)> {
        let host = self.url.host_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "endpoint missing host")
        })?;
        let port = self.url.port_or_known_default().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "endpoint missing port")
        })?;
        Ok((host.to_string(), port))
    }

    #[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
    pub(crate) fn bind_addr(&self) -> std::io::Result<String> {
        let (host, port) = self.host_port()?;
        Ok(socket_bind_addr(&host, port))
    }

    #[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
    pub(crate) fn required_param(&self, key: &str, env: &str) -> std::io::Result<String> {
        self.params
            .get(key)
            .cloned()
            .or_else(|| std::env::var(env).ok())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("missing tls parameter `{key}` (or env `{env}`)"),
                )
            })
    }

    #[cfg(feature = "tokio-tls")]
    pub(crate) fn optional_param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(String::as_str)
    }

    #[cfg(feature = "quic-s2n")]
    pub(crate) fn param_or_env_or(&self, key: &str, env: &str, default: &str) -> String {
        self.params
            .get(key)
            .cloned()
            .or_else(|| std::env::var(env).ok())
            .unwrap_or_else(|| default.to_string())
    }
}

#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
fn load_certs(path: &str) -> std::io::Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(io_other)?;
    if certs.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("no certificates found in {path}"),
        ));
    }
    Ok(certs)
}

#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
fn load_private_key(path: &str) -> std::io::Result<PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    if let Some(key) = rustls_pemfile::private_key(&mut reader).map_err(io_other)? {
        return Ok(key);
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("no private key found in {path}"),
    ))
}

#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
pub(crate) fn build_server_acceptor(
    cert_path: &str,
    key_path: &str,
) -> std::io::Result<TlsAcceptor> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(io_other)?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
pub(crate) fn build_client_config(ca_path: &str) -> std::io::Result<Arc<ClientConfig>> {
    let mut roots = RootCertStore::empty();
    for cert in load_certs(ca_path)? {
        roots.add(cert).map_err(io_other)?;
    }
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
pub(crate) fn build_client_connector(ca_path: &str) -> std::io::Result<TlsConnector> {
    Ok(TlsConnector::from(build_client_config(ca_path)?))
}

#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
pub(crate) fn server_name(
    host: &str,
    override_name: Option<&str>,
) -> std::io::Result<ServerName<'static>> {
    let name = override_name.unwrap_or(host).to_string();
    ServerName::try_from(name).map_err(io_other)
}
