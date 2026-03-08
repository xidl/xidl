use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use url::Url;

pub(crate) fn io_other<E: std::fmt::Display>(err: E) -> std::io::Error {
    std::io::Error::other(err.to_string())
}

pub(crate) fn parse_url(endpoint: &str, expected_schemes: &[&str]) -> std::io::Result<Url> {
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
    Ok(url)
}

pub(crate) fn url_host_port(url: &Url) -> std::io::Result<(String, u16)> {
    let host = url.host_str().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "endpoint missing host")
    })?;
    let port = url.port_or_known_default().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "endpoint missing port")
    })?;
    Ok((host.to_string(), port))
}

pub(crate) fn query_map(url: &Url) -> HashMap<String, String> {
    url.query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

pub(crate) fn required_param(
    params: &HashMap<String, String>,
    key: &str,
    env: &str,
) -> std::io::Result<String> {
    params
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

pub(crate) fn build_client_connector(ca_path: &str) -> std::io::Result<TlsConnector> {
    Ok(TlsConnector::from(build_client_config(ca_path)?))
}

pub(crate) fn server_name(
    host: &str,
    override_name: Option<&str>,
) -> std::io::Result<ServerName<'static>> {
    let name = override_name.unwrap_or(host).to_string();
    ServerName::try_from(name).map_err(io_other)
}
