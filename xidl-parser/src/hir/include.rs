use crate::parser::IncludeResolver;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolves `#include` paths relative to the including file on disk.
pub struct FsIncludeResolver;

impl IncludeResolver for FsIncludeResolver {
    fn resolve(
        &mut self,
        parent_path: Option<&str>,
        path: &str,
    ) -> crate::error::ParserResult<(String, String)> {
        let parent = parent_path.map(Path::new).unwrap_or_else(|| Path::new("."));
        let base = parent.parent().unwrap_or_else(|| Path::new("."));
        let resolved_path = normalize_path(&base.join(path));
        read_include(&resolved_path)
    }
}

/// Resolves `#include` paths against dependency mounts before the filesystem.
///
/// Each mount maps a package name, such as `timestamp` or `xidl/std`, to the
/// directory its library was fetched into. `#include "timestamp/lib.idl"` then
/// resolves inside that package, while every other include keeps the
/// filesystem-relative behaviour of [`FsIncludeResolver`].
pub struct MountIncludeResolver {
    mounts: BTreeMap<String, PathBuf>,
}

impl MountIncludeResolver {
    /// Creates a resolver over `mounts`, keyed by dependency package name.
    pub fn new(mounts: BTreeMap<String, PathBuf>) -> Self {
        Self { mounts }
    }

    /// Finds the longest mount whose name prefixes `path`, with its root.
    fn mount_for(&self, path: &str) -> Option<(&str, &Path)> {
        self.mounts
            .iter()
            .filter(|(name, _)| {
                path.strip_prefix(name.as_str())
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            })
            .max_by_key(|(name, _)| name.len())
            .map(|(name, root)| (name.as_str(), root.as_path()))
    }
}

impl IncludeResolver for MountIncludeResolver {
    fn resolve(
        &mut self,
        parent_path: Option<&str>,
        path: &str,
    ) -> crate::error::ParserResult<(String, String)> {
        let Some((name, root)) = self.mount_for(path) else {
            return FsIncludeResolver.resolve(parent_path, path);
        };
        let rest = path[name.len()..].trim_start_matches('/');
        let resolved_path = normalize_path(&root.join(rest));
        if !resolved_path.is_file() {
            return Err(crate::error::ParseError::Message(format!(
                "dependency include '{path}' does not exist: the '{name}' mount at '{}' \
                 resolves it to '{}'",
                root.display(),
                resolved_path.display()
            )));
        }
        read_include(&resolved_path).map_err(|err| {
            crate::error::ParseError::Message(format!(
                "dependency include '{path}' could not be read through the '{name}' mount \
                 at '{}': {err}",
                root.display()
            ))
        })
    }
}

/// Reads one resolved include target, naming it when it cannot be loaded.
fn read_include(resolved_path: &Path) -> crate::error::ParserResult<(String, String)> {
    if !resolved_path.is_file() {
        return Err(crate::error::ParseError::Message(format!(
            "include path '{}' does not exist",
            resolved_path.display()
        )));
    }

    let content = fs::read_to_string(resolved_path).map_err(|err| {
        crate::error::ParseError::Message(format!(
            "failed to read include '{}': {err}",
            resolved_path.display()
        ))
    })?;

    Ok((resolved_path.display().to_string(), content))
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    fs::canonicalize(&path).unwrap_or(path)
}
