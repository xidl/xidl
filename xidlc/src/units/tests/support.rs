use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Temporary fixture directory, removed when it drops.
pub(super) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates a uniquely named temporary directory for `name`.
    pub(super) fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("xidlc-units-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("create fixture directory");
        Self { path }
    }

    /// Root of the fixture.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Writes `content` to `relative` and returns the resulting path.
    pub(super) fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(&path, content).expect("write fixture file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
}
