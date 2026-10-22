//! `xidl.toml` library support.
//!
//! A library is a directory holding a `xidl.toml` manifest plus its IDL files.
//! This module turns command line inputs into the argument list the compiler
//! actually needs: every library argument is expanded into its input files, and
//! every declared dependency is fetched into the shared cache and mounted so
//! that `#include "<name>/<file>"` resolves inside the package.

mod command;
mod discovery;
mod error;
mod home;
mod inputs;
mod package;
mod source;
mod toolchain;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub use discovery::LoadedManifest;
pub use error::LibraryError;
pub use inputs::LibraryInputs;
pub use package::{GitPackageFetcher, PackageFetcher, PackageStore};
pub use source::{LOCAL_HOST, PackageSource};
pub use toolchain::{Toolchain, ToolchainSource};

/// Generator property that carries dependency mounts into the parser.
pub const MOUNT_PROPERTY: &str = "xidl_mounts";

/// Builds the include resolver for `mounts`.
///
/// Include and import paths resolve inside a dependency package when their
/// first segment names a mount, and relative to the including file otherwise.
pub fn include_resolver(
    mounts: &BTreeMap<String, PathBuf>,
) -> Box<dyn xidl_parser::parser::IncludeResolver> {
    if mounts.is_empty() {
        Box::new(xidl_parser::hir::FsIncludeResolver)
    } else {
        Box::new(xidl_parser::hir::MountIncludeResolver::new(mounts.clone()))
    }
}

/// Inputs resolved from the command line once every manifest is expanded.
#[derive(Debug, Clone, Default)]
pub struct ResolvedInputs {
    files: Vec<PathBuf>,
    mounts: BTreeMap<String, PathBuf>,
    manifests: Vec<LoadedManifest>,
}

impl ResolvedInputs {
    /// Compiler inputs, in argument order.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Dependency mounts, keyed by package name.
    pub fn mounts(&self) -> &BTreeMap<String, PathBuf> {
        &self.mounts
    }

    /// Manifests that contributed inputs.
    pub fn manifests(&self) -> &[LoadedManifest] {
        &self.manifests
    }

    /// Mounts rendered as the property the `hir` generator reads.
    pub fn mount_property(&self) -> serde_json::Value {
        let mounts = self
            .mounts
            .iter()
            .map(|(name, root)| {
                (
                    name.clone(),
                    serde_json::Value::String(root.to_string_lossy().into_owned()),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        serde_json::Value::Object(mounts)
    }

    /// Rejects manifests whose `compiler` range the running compiler misses.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Toolchain`] naming the manifest and the range it
    /// asks for, so an in-process build never compiles with the wrong compiler.
    pub fn check_compiler(&self) -> Result<(), LibraryError> {
        let current = Toolchain::current_version();
        for loaded in &self.manifests {
            let manifest = loaded.manifest();
            let Some(requirement) = manifest.compiler() else {
                continue;
            };
            if manifest.accepts_compiler(&current) {
                continue;
            }
            return Err(LibraryError::Toolchain {
                requirement: requirement.to_string(),
                reason: format!(
                    "{} requires it but the running xidlc is {current}; run the xidlc \
                     CLI so it can install and switch to the matching toolchain",
                    loaded.path().display()
                ),
            });
        }
        Ok(())
    }
}

/// Expands command line inputs and fetches the dependencies they declare.
pub struct LibraryLoader {
    packages: PackageStore,
}

impl LibraryLoader {
    /// Creates a loader over the default package cache.
    pub fn new() -> Self {
        Self {
            packages: PackageStore::new(),
        }
    }

    /// Creates a loader that fetches dependencies with `packages`.
    pub fn with_packages(packages: PackageStore) -> Self {
        Self { packages }
    }

    /// Manifests declared by `paths`, without fetching anything.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Manifest`] when a discovered manifest is invalid.
    pub fn discover(&self, paths: &[PathBuf]) -> Result<Vec<LoadedManifest>, LibraryError> {
        let mut manifests = Vec::new();
        for path in paths {
            if let Some(manifest) = LoadedManifest::discover(path)? {
                manifests.push(manifest);
            }
        }
        Ok(manifests)
    }

    /// Expands `paths` into compiler inputs plus dependency mounts.
    ///
    /// A path that is not a library is passed through unchanged, so plain
    /// `.idl` arguments keep their current meaning.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NoInputs`] when a library matches no IDL files,
    /// and [`LibraryError::Fetch`] when a dependency cannot be materialized.
    pub fn resolve(&self, paths: &[PathBuf]) -> Result<ResolvedInputs, LibraryError> {
        let mut files = Vec::new();
        let mut seen = BTreeSet::new();
        let mut mounts = BTreeMap::new();
        let mut manifests = Vec::new();

        for path in paths {
            let Some(loaded) = LoadedManifest::discover(path)? else {
                if seen.insert(path.clone()) {
                    files.push(path.clone());
                }
                continue;
            };

            for file in loaded.inputs()? {
                if seen.insert(file.clone()) {
                    files.push(file);
                }
            }
            for (name, dependency) in loaded.manifest().dependencies() {
                if mounts.contains_key(name) {
                    continue;
                }
                let mount = match dependency.path() {
                    Some(path) => loaded.resolve_path_dependency(name, path)?,
                    None => self.packages.ensure(name, dependency)?,
                };
                mounts.insert(name.clone(), mount);
            }
            manifests.push(loaded);
        }

        Ok(ResolvedInputs {
            files,
            mounts,
            manifests,
        })
    }
}

impl Default for LibraryLoader {
    fn default() -> Self {
        Self::new()
    }
}
