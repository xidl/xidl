#[cfg(unix)]
use std::path::{Path, PathBuf};

use std::net::SocketAddr;

use super::{Listener, Stream, loopback_peer_addr};

#[cfg(unix)]
pub struct IpcListener {
    path: PathBuf,
    inner: tokio::net::UnixListener,
}

#[cfg(unix)]
impl IpcListener {
    /// Binds a unix domain socket at `path`, reclaiming a stale socket file
    /// only when no live listener still accepts on it.
    pub fn bind(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match tokio::net::UnixListener::bind(&path) {
            Ok(inner) => Ok(Self { path, inner }),
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
                Self::reclaim_stale_socket(&path)?;
                let inner = tokio::net::UnixListener::bind(&path)?;
                Ok(Self { path, inner })
            }
            Err(err) => Err(err),
        }
    }

    /// Removes a stale socket file at `path` when no live listener answers.
    ///
    /// A unix socket path survives in the filesystem after its listener
    /// exits, so a fresh bind reports `AddrInUse` even though nothing is
    /// listening. This probes the path first and only deletes the file when
    /// the probe is refused, preserving a live server that owns the path.
    fn reclaim_stale_socket(path: &Path) -> std::io::Result<()> {
        if std::os::unix::net::UnixStream::connect(path).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("ipc endpoint already in use: {}", path.display()),
            ));
        }
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
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
        Ok((Box::new(stream), loopback_peer_addr()))
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
