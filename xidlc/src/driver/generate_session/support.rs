use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Codegen, CodegenClient};
use semver::{Version, VersionReq};
use std::future::Future;

pub(super) async fn retry_connect<T, F, Fut>(mut connect: F, error_message: String) -> IdlcResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::io::Result<T>>,
{
    let mut last_err = None;
    for _ in 0..50 {
        match connect().await {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                last_err = Some(err);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }
    let err = last_err.unwrap_or_else(|| std::io::Error::other(error_message));
    Err(IdlcError::rpc(err.to_string()))
}

pub(super) fn random_inproc_endpoint(lang: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_nanos();
    format!("xidlc.codegen.{lang}.{nanos}")
}

#[cfg(unix)]
pub(super) fn rpc_endpoint(lang: &str) -> IdlcResult<String> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_nanos();
    let path = std::path::Path::new("/tmp").join(format!("xidlc.{lang}.{nanos}.sock"));
    let path = path
        .into_os_string()
        .into_string()
        .map_err(|_| IdlcError::rpc("invalid ipc socket path".to_string()))?;
    Ok(format!("ipc://{path}"))
}

#[cfg(windows)]
pub(super) fn rpc_endpoint(_lang: &str) -> IdlcResult<String> {
    let probe = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|err| IdlcError::rpc(err.to_string()))?;
    let addr = probe
        .local_addr()
        .map_err(|err| IdlcError::rpc(err.to_string()))?;
    drop(probe);
    Ok(format!("tcp://{addr}"))
}

pub(super) async fn verify_engine_version<S>(client: &CodegenClient<S>) -> IdlcResult<()>
where
    S: xidl_jsonrpc::transport::Stream + Unpin + Send,
{
    let engine_req: String = client
        .get_engine_version()
        .await
        .map_err(|err| IdlcError::rpc(err.to_string()))?;
    let req = VersionReq::parse(&engine_req).map_err(|err| {
        IdlcError::rpc(format!(
            "invalid engine version requirement \"{engine_req}\": {err}"
        ))
    })?;
    let version = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|err| {
        IdlcError::rpc(format!(
            "invalid xidlc version \"{}\": {err}",
            env!("CARGO_PKG_VERSION")
        ))
    })?;
    if !req.matches(&version) {
        return Err(IdlcError::rpc(format!(
            "xidlc {version} is not compatible with engine requirement {engine_req}"
        )));
    }
    Ok(())
}
