//! Library manifest (`xidl.toml`) parsing.
//!
//! A manifest gives a directory of IDL files an identity (a package `name` and
//! `version`), the inputs that make up the library (`files` / `exclude`), the
//! `compiler` version range it needs, and the other libraries it declares under
//! `[dependencies]`.

mod dependency;
mod error;
mod parse;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::Path;

pub use dependency::{Dependency, Revision};
pub use error::ManifestError;
use semver::{Version, VersionReq};

/// File name of the library manifest.
pub const MANIFEST_FILE_NAME: &str = "xidl.toml";

/// A validated `xidl.toml` library manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    name: String,
    version: Version,
    files: Option<Vec<String>>,
    exclude: Vec<String>,
    compiler: Option<VersionReq>,
    dependencies: BTreeMap<String, Dependency>,
}

impl Manifest {
    /// Loads and validates the manifest stored at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Read`] when the file cannot be read and the
    /// parse errors of [`Manifest::parse`] when its contents are invalid.
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let text = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(path, &text)
    }

    /// Parses manifest `text`; `path` only labels diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Deserialize`] for text that is not a TOML
    /// object with the expected fields, and [`ManifestError::InvalidField`] for
    /// values that are present but unusable, such as a `compiler` range that is
    /// not a semantic version requirement.
    pub fn parse(path: &Path, text: &str) -> Result<Self, ManifestError> {
        parse::from_str(path, text)
    }

    /// Package identity used by `import` and by dependency declarations.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The library's own release version.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Declared input patterns, or `None` to use the manifest directory.
    pub fn files(&self) -> Option<&[String]> {
        self.files.as_deref()
    }

    /// Patterns removed from the resolved input set.
    pub fn exclude(&self) -> &[String] {
        &self.exclude
    }

    /// Compiler version range the library needs, when it declares one.
    pub fn compiler(&self) -> Option<&VersionReq> {
        self.compiler.as_ref()
    }

    /// Libraries this one depends on, keyed by package name.
    pub fn dependencies(&self) -> &BTreeMap<String, Dependency> {
        &self.dependencies
    }

    /// Reports whether `version` satisfies this manifest's compiler range.
    ///
    /// A manifest without a `compiler` key accepts every compiler version.
    pub fn accepts_compiler(&self, version: &Version) -> bool {
        self.compiler
            .as_ref()
            .is_none_or(|required| required.matches(version))
    }
}
