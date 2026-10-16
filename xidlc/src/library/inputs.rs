use super::error::LibraryError;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use xidl_parser::manifest::Manifest;

/// Default input pattern of a library: `.idl` files beside the manifest.
const DEFAULT_PATTERN: &str = "*.idl";

/// The input files one manifest directory contributes to a compiler run.
#[derive(Debug, Clone)]
pub struct LibraryInputs {
    dir: PathBuf,
    patterns: Vec<String>,
    exclude: Vec<String>,
}

impl LibraryInputs {
    /// Collects the input patterns of `manifest`, rooted at `dir`.
    ///
    /// A manifest without `files` uses [`DEFAULT_PATTERN`], so a library is
    /// exactly the IDL files that sit next to its manifest.
    pub fn of(manifest: &Manifest, dir: &Path) -> Self {
        let patterns = match manifest.files() {
            Some(files) if !files.is_empty() => files.to_vec(),
            _ => vec![DEFAULT_PATTERN.to_string()],
        };
        Self {
            dir: dir.to_path_buf(),
            patterns,
            exclude: manifest.exclude().to_vec(),
        }
    }

    /// Resolves the patterns into a sorted, deduplicated file list.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NoInputs`] when nothing matched, so a typo in
    /// `files` fails loudly instead of compiling an empty library.
    pub fn resolve(&self) -> Result<Vec<PathBuf>, LibraryError> {
        let mut files = BTreeSet::new();
        for pattern in &self.patterns {
            files.extend(self.matches(pattern)?);
        }
        for pattern in &self.exclude {
            for path in self.matches(pattern)? {
                files.remove(&path);
            }
        }

        if files.is_empty() {
            return Err(LibraryError::NoInputs {
                dir: self.dir.clone(),
                patterns: self.patterns.clone(),
            });
        }
        Ok(files.into_iter().collect())
    }

    /// Expands one pattern relative to the manifest directory.
    fn matches(&self, pattern: &str) -> Result<Vec<PathBuf>, LibraryError> {
        let joined = self.dir.join(pattern);
        let Some(joined) = joined.to_str() else {
            return Err(LibraryError::Source {
                name: self.dir.display().to_string(),
                src: pattern.to_string(),
                reason: "path is not valid UTF-8".to_string(),
            });
        };
        let entries = glob::glob(joined).map_err(|err| LibraryError::Source {
            name: self.dir.display().to_string(),
            src: pattern.to_string(),
            reason: err.to_string(),
        })?;
        let mut matched = Vec::new();
        for entry in entries {
            let path = entry.map_err(|err| LibraryError::Source {
                name: self.dir.display().to_string(),
                src: pattern.to_string(),
                reason: err.to_string(),
            })?;
            if path.is_file() {
                matched.push(path);
            }
        }
        Ok(matched)
    }
}
