//! A compilation generator owns exactly the output paths in its published manifest.

use super::File;
use crate::error::{IdlcError, IdlcResult};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct CompilationOutput<'a> {
    directory: &'a str,
    lang: &'a str,
}

impl<'a> CompilationOutput<'a> {
    pub(super) fn new(directory: &'a str, lang: &'a str) -> Self {
        Self { directory, lang }
    }

    pub(super) fn write(&self, files: Vec<File>) -> IdlcResult<()> {
        let mut paths = BTreeSet::new();
        for file in &files {
            Self::validate_path(&file.path)?;
            if !paths.insert(file.path.clone()) {
                return Err(IdlcError::rpc(format!(
                    "compilation output contains duplicate path '{}'",
                    file.path
                )));
            }
        }
        if self.directory == "-" {
            for file in files {
                println!("{}", file.content);
            }
            return Ok(());
        }
        let directory = Path::new(self.directory);
        let name: String = self
            .lang
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        std::fs::create_dir_all(directory)?;
        let directory = std::fs::canonicalize(directory)?;
        let manifest = directory.join(format!(".xidlc-{name}-outputs.json"));
        Self::validate_destination(&directory, &manifest)?;
        let previous: BTreeSet<String> = if manifest.exists() {
            serde_json::from_slice(&std::fs::read(&manifest)?)?
        } else {
            BTreeSet::new()
        };
        for path in &previous {
            Self::validate_path(path)?;
        }
        for path in paths.union(&previous) {
            let destination = directory.join(path);
            if destination == manifest {
                return Err(IdlcError::rpc(
                    "compilation output conflicts with its ownership manifest",
                ));
            }
            Self::validate_destination(&directory, &destination)?;
            if paths.contains(path) && !previous.contains(path) && destination.exists() {
                return Err(IdlcError::rpc(format!(
                    "compilation output would overwrite a user-owned file: '{path}'"
                )));
            }
            for parent in Path::new(path).ancestors().skip(1) {
                if paths.contains(&parent.to_string_lossy().into_owned()) {
                    return Err(IdlcError::rpc(format!(
                        "compilation output file conflicts with a parent path: '{path}'"
                    )));
                }
            }
        }
        for file in files {
            let path = directory.join(&file.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, file.content)?;
        }
        for stale in previous.difference(&paths) {
            let path = directory.join(stale);
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        std::fs::write(manifest, serde_json::to_vec_pretty(&paths)?)?;
        Ok(())
    }

    fn validate_destination(directory: &Path, destination: &Path) -> IdlcResult<()> {
        let relative = destination.strip_prefix(directory).map_err(|_| {
            IdlcError::rpc("compilation output destination is outside its output directory")
        })?;
        let mut path = directory.to_path_buf();
        for part in relative.components() {
            path.push(part);
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            if metadata.file_type().is_symlink() {
                return Err(IdlcError::rpc(format!(
                    "compilation output path crosses a symbolic link: '{}'",
                    path.display()
                )));
            }
            if (path == destination && !metadata.is_file())
                || (path != destination && !metadata.is_dir())
            {
                return Err(IdlcError::rpc(format!(
                    "compilation output path conflicts with an existing file or directory: '{}'",
                    path.display()
                )));
            }
        }
        Ok(())
    }

    fn validate_path(path: &str) -> IdlcResult<()> {
        if path.is_empty()
            || Path::new(path)
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(IdlcError::rpc(format!(
                "compilation output requires a relative file path: '{path}'"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
