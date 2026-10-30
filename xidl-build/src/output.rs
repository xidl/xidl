//! Isolated generation followed by publication on the destination filesystem.

use std::fs;
use std::path::{Path, PathBuf};

use crate::Error;

/// A private generation directory and the file it will publish.
pub(crate) struct StagedOutput {
    directory: tempfile::TempDir,
    destination: PathBuf,
}

impl StagedOutput {
    /// Creates staging beside the destination so publication can use rename.
    pub(crate) fn new(destination: PathBuf) -> Result<Self, Error> {
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let directory = tempfile::Builder::new()
            .prefix(".xidl-build-stage-")
            .tempdir_in(parent)?;
        Ok(Self {
            directory,
            destination,
        })
    }

    /// Directory owned exclusively by this generation request.
    pub(crate) fn directory(&self) -> &Path {
        self.directory.path()
    }

    /// Publishes supporting files before replacing the entry document.
    pub(crate) fn publish(self, source: &Path) -> Result<(), Error> {
        let parent = self
            .destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if let Some(name) = self.destination.file_name() {
            let companion = self.directory().join(name);
            if companion != source && companion.exists() {
                return Err(Error::Config(format!(
                    "output filename '{}' conflicts with a generated supporting document",
                    self.destination.display()
                )));
            }
        }
        Self::publish_directory(self.directory(), parent, source)?;
        fs::rename(source, &self.destination)?;
        Ok(())
    }

    fn publish_directory(from: &Path, to: &Path, entry: &Path) -> Result<(), Error> {
        fs::create_dir_all(to)?;
        for item in fs::read_dir(from)? {
            let item = item?;
            let source = item.path();
            if source == entry {
                continue;
            }
            let destination = to.join(item.file_name());
            if item.file_type()?.is_dir() {
                Self::publish_directory(&source, &destination, entry)?;
            } else {
                fs::rename(source, destination)?;
            }
        }
        Ok(())
    }
}
