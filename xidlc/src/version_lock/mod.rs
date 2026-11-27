//! Version lock file generation and drift detection for frozen API versions.

#[cfg(test)]
mod tests;

use crate::error::{IdlcError, IdlcResult};
use crate::version_diff::VersionDiff;
use crown::hash::sha256::sum256;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const LOCK_SCHEMA_VERSION: u32 = 1;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Serialized version lock file containing one stable fingerprint per version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LockDocument {
    pub(crate) schema_version: u32,
    pub(crate) versions: BTreeMap<String, LockedVersion>,
}

/// Fingerprint metadata recorded for one version in a lock file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LockedVersion {
    pub(crate) id: String,
    pub(crate) fingerprint: String,
}

/// One detected discrepancy between a lock file and the current IDL contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LockDrift {
    /// A locked version's canonical projection changed.
    Changed {
        version: String,
        id: String,
        expected: String,
        actual: String,
    },
    /// A locked version was removed from the contract.
    Removed { version: String, id: String },
    /// The contract declares a version absent from the lock file (`--strict`).
    Unlocked { version: String, id: String },
}

impl LockDrift {
    /// Renders a human-readable diagnostic line for this drift entry.
    pub(crate) fn format_line(&self) -> String {
        match self {
            Self::Changed {
                version,
                id,
                expected,
                actual,
            } => format!("version '{version}' ({id}) changed: locked {expected}, current {actual}"),
            Self::Removed { version, id } => {
                format!("version '{version}' ({id}) was removed from the contract")
            }
            Self::Unlocked { version, id } => {
                format!("version '{version}' ({id}) is not present in the lock file")
            }
        }
    }
}

/// Outcome of validating a contract against its version lock file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LockCheckReport {
    pub(crate) drifts: Vec<LockDrift>,
}

impl LockCheckReport {
    /// Returns true when no frozen version drifted.
    pub(crate) fn is_clean(&self) -> bool {
        self.drifts.is_empty()
    }

    /// Formats all detected drifts as a newline-terminated message.
    pub(crate) fn render(&self, contract: &Path, lock_path: &Path) -> String {
        let mut out = format!(
            "version lock mismatch for '{}' against '{}':\n",
            contract.display(),
            lock_path.display()
        );
        for drift in &self.drifts {
            out.push_str("  - ");
            out.push_str(&drift.format_line());
            out.push('\n');
        }
        out
    }
}

/// Outcome of writing or updating a version lock file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LockWriteReport {
    pub(crate) lock_path: PathBuf,
    pub(crate) document: LockDocument,
    pub(crate) drifts: Vec<LockDrift>,
}

impl LockWriteReport {
    /// Returns true when the lock file was written with no unforced drifts.
    pub(crate) fn is_written(&self) -> bool {
        self.drifts.is_empty()
    }

    /// Formats unforced drift errors that blocked overwriting the lock file.
    pub(crate) fn render_drifts(&self, contract: &Path) -> String {
        let report = LockCheckReport {
            drifts: self.drifts.clone(),
        };
        format!(
            "{}re-run with --force to update locked fingerprints intentionally.\n",
            report.render(contract, &self.lock_path)
        )
    }
}

/// Computes and verifies stable per-version fingerprints for a versioned IDL file.
pub(crate) struct VersionLock {
    diff: VersionDiff,
    versions: Option<BTreeSet<String>>,
}

impl VersionLock {
    /// Resolves a versioned IDL contract using the same pipeline as `version diff`.
    pub(crate) fn from_file(path: &Path) -> IdlcResult<Self> {
        let diff = VersionDiff::from_file(path)?;
        Ok(Self {
            diff,
            versions: None,
        })
    }

    /// Limits lock and check to the given version values when supplied.
    pub(crate) fn with_versions(mut self, values: Vec<String>) -> Self {
        self.versions = (!values.is_empty()).then(|| values.into_iter().collect());
        self
    }

    fn selects(&self, value: &str) -> bool {
        self.versions
            .as_ref()
            .is_none_or(|versions| versions.contains(value))
    }

    fn validate_selection(
        &self,
        current: &LockDocument,
        existing: Option<&LockDocument>,
    ) -> IdlcResult<()> {
        if let Some(versions) = &self.versions
            && let Some(unknown) = versions.iter().find(|value| {
                !current.versions.contains_key(*value)
                    && !existing.is_some_and(|document| document.versions.contains_key(*value))
            })
        {
            return Err(IdlcError::fmt(format!(
                "version selection: version '{unknown}' is not declared in the contract"
            )));
        }
        Ok(())
    }

    /// Derives the default lock file path next to `idl`.
    pub(crate) fn default_lock_path(idl: &Path) -> PathBuf {
        let stem = idl
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("contract");
        let file_name = format!("{stem}.versions.lock.json");
        match idl.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.join(file_name),
            _ => PathBuf::from(file_name),
        }
    }

    /// Computes a `sha256:<hex>` digest string using `crown`.
    pub(crate) fn fingerprint(input: &[u8]) -> String {
        let digest = sum256(input);
        let mut out = String::with_capacity(7 + 64);
        out.push_str("sha256:");
        for byte in digest {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    /// Builds a deterministic lock document for the selected versions.
    fn document(&self) -> IdlcResult<LockDocument> {
        let record = self.diff.record();
        let parameter_line = match record.parameter() {
            Some(param) => format!("{}:{}", param.location.as_str(), param.name),
            None => "none".to_string(),
        };
        let mut versions = BTreeMap::new();
        for (version_id, definition) in record.timeline().iter() {
            if !self.selects(&definition.value) {
                continue;
            }
            let projection = self.diff.formatted_projection(version_id)?;
            let mut canonical = format!(
                "xidl-version-lock-v1\nversion-value: {}\nversion-parameter: {}\n---\n{}",
                definition.value, parameter_line, projection
            );
            for (unit_path, unit_source) in self.diff.imported_units() {
                let _ = write!(canonical, "\n--- import {unit_path} ---\n{unit_source}");
            }
            let fingerprint = Self::fingerprint(canonical.as_bytes());
            versions.insert(
                definition.value.clone(),
                LockedVersion {
                    id: definition.ident.clone(),
                    fingerprint,
                },
            );
        }
        Ok(LockDocument {
            schema_version: LOCK_SCHEMA_VERSION,
            versions,
        })
    }

    /// Writes a lock file atomically, refusing to overwrite drifted existing entries unless `force` is set.
    pub(crate) fn write_lock(&self, lock_path: &Path, force: bool) -> IdlcResult<LockWriteReport> {
        let current = self.document()?;
        self.validate_selection(&current, None)?;
        if !force && let Some(mut existing) = Self::read_document_if_exists(lock_path)? {
            existing.versions.retain(|value, _| self.selects(value));
            let report = Self::compare(&existing, &current, false);
            if !report.is_clean() {
                return Ok(LockWriteReport {
                    lock_path: lock_path.to_path_buf(),
                    document: current,
                    drifts: report.drifts,
                });
            }
        }
        let parent = lock_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
        if let Some(parent) = parent {
            std::fs::create_dir_all(parent).map_err(|error| {
                IdlcError::fmt(format!(
                    "version lock: cannot create directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }
        let mut encoded = serde_json::to_string_pretty(&current)?;
        encoded.push('\n');
        Self::write_atomic(lock_path, encoded.as_bytes())?;
        Ok(LockWriteReport {
            lock_path: lock_path.to_path_buf(),
            document: current,
            drifts: Vec::new(),
        })
    }

    /// Verifies that every version in `lock_path` still matches the contract.
    pub(crate) fn check(&self, lock_path: &Path, strict: bool) -> IdlcResult<LockCheckReport> {
        let mut existing = Self::read_document(lock_path)?;
        let current = self.document()?;
        self.validate_selection(&current, Some(&existing))?;
        if self.versions.is_some() {
            let declared: BTreeSet<&str> = self
                .diff
                .record()
                .timeline()
                .iter()
                .map(|(_, definition)| definition.value.as_str())
                .collect();
            // Temporarily omitted versions can still be declared. Keep only actual removals.
            existing
                .versions
                .retain(|value, _| self.selects(value) || !declared.contains(value.as_str()));
        }
        Ok(Self::compare(
            &existing,
            &current,
            strict || self.versions.is_some(),
        ))
    }

    fn read_document_if_exists(lock_path: &Path) -> IdlcResult<Option<LockDocument>> {
        match std::fs::read_to_string(lock_path) {
            Ok(raw) => Self::parse_document(lock_path, &raw).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(IdlcError::fmt(format!(
                "version lock: cannot read '{}': {error}",
                lock_path.display()
            ))),
        }
    }

    fn read_document(lock_path: &Path) -> IdlcResult<LockDocument> {
        let raw = std::fs::read_to_string(lock_path).map_err(|error| {
            IdlcError::fmt(format!(
                "version lock: cannot read '{}': {error}",
                lock_path.display()
            ))
        })?;
        Self::parse_document(lock_path, &raw)
    }

    fn parse_document(lock_path: &Path, raw: &str) -> IdlcResult<LockDocument> {
        let document: LockDocument = serde_json::from_str(raw).map_err(|error| {
            IdlcError::fmt(format!(
                "version lock: invalid JSON in '{}': {error}",
                lock_path.display()
            ))
        })?;
        if document.schema_version != LOCK_SCHEMA_VERSION {
            return Err(IdlcError::fmt(format!(
                "version lock: unsupported schema_version {} in '{}' (expected {})",
                document.schema_version,
                lock_path.display(),
                LOCK_SCHEMA_VERSION
            )));
        }
        Ok(document)
    }

    fn write_atomic(lock_path: &Path, bytes: &[u8]) -> IdlcResult<()> {
        let file_name = lock_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("versions.lock.json");
        let seq = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!(".{file_name}.tmp.{}.{seq}", std::process::id());
        let temp_path = match lock_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.join(temp_name),
            _ => PathBuf::from(temp_name),
        };
        let cleanup = scopeguard::guard(temp_path.clone(), |path| {
            let _ = std::fs::remove_file(path);
        });
        std::fs::write(&temp_path, bytes).map_err(|error| {
            IdlcError::fmt(format!(
                "version lock: cannot write '{}': {error}",
                lock_path.display()
            ))
        })?;
        std::fs::rename(&temp_path, lock_path).map_err(|error| {
            IdlcError::fmt(format!(
                "version lock: cannot replace '{}': {error}",
                lock_path.display()
            ))
        })?;
        scopeguard::ScopeGuard::into_inner(cleanup);
        Ok(())
    }

    fn compare(existing: &LockDocument, current: &LockDocument, strict: bool) -> LockCheckReport {
        let mut drifts = Vec::new();
        for (version, locked) in &existing.versions {
            match current.versions.get(version) {
                None => drifts.push(LockDrift::Removed {
                    version: version.clone(),
                    id: locked.id.clone(),
                }),
                Some(actual) if actual.fingerprint != locked.fingerprint => {
                    drifts.push(LockDrift::Changed {
                        version: version.clone(),
                        id: actual.id.clone(),
                        expected: locked.fingerprint.clone(),
                        actual: actual.fingerprint.clone(),
                    });
                }
                Some(_) => {}
            }
        }
        if strict {
            for (version, actual) in &current.versions {
                if !existing.versions.contains_key(version) {
                    drifts.push(LockDrift::Unlocked {
                        version: version.clone(),
                        id: actual.id.clone(),
                    });
                }
            }
        }
        LockCheckReport { drifts }
    }
}
