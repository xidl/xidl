use super::error::LibraryError;
use super::inputs::LibraryInputs;
use std::path::{Path, PathBuf};
use xidl_parser::manifest::{MANIFEST_FILE_NAME, Manifest};

/// A manifest discovered among the compiler inputs.
#[derive(Debug, Clone)]
pub struct LoadedManifest {
    path: PathBuf,
    dir: PathBuf,
    manifest: Manifest,
}

impl LoadedManifest {
    /// Loads the library `path` names, or `None` when it names a plain input.
    ///
    /// A path is a library when it is a `xidl.toml` file or a directory that
    /// holds one; every other path stays an ordinary IDL input.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Manifest`] when the manifest exists but cannot
    /// be read or validated.
    pub fn discover(path: &Path) -> Result<Option<Self>, LibraryError> {
        let manifest_path = if path.is_dir() {
            path.join(MANIFEST_FILE_NAME)
        } else if path
            .file_name()
            .is_some_and(|name| name == MANIFEST_FILE_NAME)
        {
            path.to_path_buf()
        } else {
            return Ok(None);
        };
        if !manifest_path.is_file() {
            return Ok(None);
        }

        let manifest = Manifest::load(&manifest_path)?;
        let dir = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Ok(Some(Self {
            path: manifest_path,
            dir,
            manifest,
        }))
    }

    /// Path of the `xidl.toml` that was loaded.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Directory holding the manifest and its IDL files.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Parsed manifest contents.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The IDL files this library contributes to the compiler run.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NoInputs`] when the manifest's patterns match
    /// nothing.
    pub fn inputs(&self) -> Result<Vec<PathBuf>, LibraryError> {
        LibraryInputs::of(&self.manifest, &self.dir).resolve()
    }
}
