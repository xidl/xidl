#[cfg(unix)]
use std::path::{Path, PathBuf};

use std::net::SocketAddr;

use super::{Listener, Stream};

#[cfg(unix)]
pub struct IpcListener {
    path: PathBuf,
    inner: tokio::net::UnixListener,
}

#[cfg(unix)]
impl IpcListener {
    pub fn bind(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        let inner = tokio::net::UnixListener::bind(&path)?;
        Ok(Self { path, inner })
    }
}

#[cfg(unix)]
impl Drop for IpcListener {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {}
        }
    }
}

#[cfg(unix)]
#[async_trait::async_trait]
impl Listener for IpcListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn Stream + Unpin + Send + 'static>, SocketAddr)> {
        let (stream, _peer) = self.inner.accept().await?;
        Ok((Box::new(stream), SocketAddr::from(([127, 0, 0, 1], 0))))
    }

    fn endpoint(&self) -> Option<String> {
        Some(format!("ipc://{}", self.path.display()))
    }
}

#[cfg(unix)]
pub async fn connect_ipc(path: &str) -> std::io::Result<Box<dyn Stream + Unpin + Send + 'static>> {
    let stream = tokio::net::UnixStream::connect(path).await?;
    Ok(Box::new(stream))
}
