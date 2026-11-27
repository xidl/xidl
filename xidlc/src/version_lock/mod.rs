//! Version lock file generation and drift detection for frozen API versions.

#[cfg(test)]
mod tests;

mod openapi;
mod report;
mod wire;

use crate::error::{IdlcError, IdlcResult};
use crate::library::ResolvedInputs;
use crate::units::UnitGraph;
use crate::version_diff::VersionDiff;
use crown::hash::sha256::sum256;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use openapi::OpenApiFingerprints;
#[cfg(test)]
use report::LockDrift;
use report::{Evidence, LockCheckReport, LockWriteReport};

const LOCK_SCHEMA_VERSION: u32 = 1;
const LOCK_ALGORITHM_VERSION: u32 = 1;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Optional generated artifact to fingerprint alongside the IDL projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VersionArtifact {
    /// The OpenAPI document and the schemas reachable from its operations.
    OpenApi,
}

/// Serialized version lock file containing one stable fingerprint per version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LockDocument {
    pub(crate) schema_version: u32,
    pub(crate) algorithm_version: Option<u32>,
    pub(crate) versions: BTreeMap<String, LockedVersion>,
}

impl LockDocument {
    fn requires_relock(&self, path: &Path, force: bool) -> IdlcResult<bool> {
        match self.algorithm_version {
            Some(version) if version == LOCK_ALGORITHM_VERSION => Ok(false),
            Some(version) if version > LOCK_ALGORITHM_VERSION => Err(IdlcError::fmt(format!(
                "version lock: algorithm_version {version} in '{}' requires a newer compiler",
                path.display()
            ))),
            _ if force => Ok(true),
            _ => Err(IdlcError::fmt(format!(
                "version lock: '{}' uses an older or missing algorithm version; regenerate it with version lock --force",
                path.display()
            ))),
        }
    }

    fn has_openapi(&self, value: &str) -> bool {
        self.versions
            .get(value)
            .is_some_and(|entry| entry.openapi_fingerprint.is_some())
    }
}

/// Fingerprint metadata recorded for one version in a lock file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LockedVersion {
    pub(crate) id: String,
    pub(crate) fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) openapi_fingerprint: Option<String>,
}

/// Computes and verifies stable per-version fingerprints for a versioned IDL file.
pub(crate) struct VersionLock {
    diff: VersionDiff,
    versions: Option<BTreeSet<String>>,
    graph: UnitGraph,
    resolved: ResolvedInputs,
    artifact: Option<VersionArtifact>,
}

impl VersionLock {
    /// Resolves a versioned IDL contract using the same pipeline as `version diff`.
    pub(crate) fn from_file(path: &Path) -> IdlcResult<Self> {
        let (diff, graph, resolved) = VersionDiff::load(path)?;
        Ok(Self {
            diff,
            versions: None,
            graph,
            resolved,
            artifact: None,
        })
    }

    /// Limits lock and check to the given version values when supplied.
    pub(crate) fn with_versions(mut self, versions: Option<BTreeSet<String>>) -> Self {
        self.versions = versions;
        self
    }

    /// Selects explicit evidence from the generated artifact.
    pub(crate) fn with_artifact(mut self, artifact: Option<VersionArtifact>) -> Self {
        self.artifact = artifact;
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
    fn document(&self, needs_openapi: impl Fn(&str) -> bool) -> IdlcResult<LockDocument> {
        let record = self.diff.record();
        let parameter_line = match record.parameter() {
            Some(param) => format!("{}:{}", param.location.as_str(), param.name),
            None => "none".to_string(),
        };
        let mut versions = BTreeMap::new();
        let mut openapi = None;
        for (version_id, definition) in record.timeline().iter() {
            if !self.selects(&definition.value) {
                continue;
            }
            let projection = self.diff.formatted_projection(version_id)?;
            let mut canonical = format!(
                "xidl-version-lock\nversion-value: {}\nversion-parameter: {}\n---\n{}",
                definition.value, parameter_line, projection
            );
            for (unit_path, unit_source) in self.diff.imported_units() {
                let _ = write!(canonical, "\n--- import {unit_path} ---\n{unit_source}");
            }
            let fingerprint = Self::fingerprint(canonical.as_bytes());
            let openapi_fingerprint = if needs_openapi(&definition.value) {
                let generator = match &mut openapi {
                    Some(generator) => generator,
                    slot => slot.insert(OpenApiFingerprints::new(&self.graph, &self.resolved)?),
                };
                Some(generator.fingerprint(&definition.value)?)
            } else {
                None
            };
            versions.insert(
                definition.value.clone(),
                LockedVersion {
                    id: definition.ident.clone(),
                    fingerprint,
                    openapi_fingerprint,
                },
            );
        }
        Ok(LockDocument {
            schema_version: LOCK_SCHEMA_VERSION,
            algorithm_version: Some(LOCK_ALGORITHM_VERSION),
            versions,
        })
    }

    /// Writes a lock file atomically, refusing to overwrite drifted existing entries unless `force` is set.
    pub(crate) fn write_lock(&self, lock_path: &Path, force: bool) -> IdlcResult<LockWriteReport> {
        let existing = Self::read_document_if_exists(lock_path)?;
        let upgrading = match &existing {
            Some(document) => document.requires_relock(lock_path, force)?,
            None => false,
        };
        let current = self.document(|value| {
            self.artifact.is_some() || existing.as_ref().is_some_and(|doc| doc.has_openapi(value))
        })?;
        self.validate_selection(&current, None)?;
        if let Some(mut existing) = existing
            && !upgrading
            && !force
        {
            existing.versions.retain(|value, _| self.selects(value));
            let report = LockCheckReport::compare(&existing, &current, Evidence::Recorded, false)?;
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
    pub(crate) fn check(&self, lock_path: &Path) -> IdlcResult<LockCheckReport> {
        let mut existing = Self::read_document(lock_path)?;
        existing.requires_relock(lock_path, false)?;
        let current = self.document(|value| existing.has_openapi(value))?;
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
        let evidence = if self.artifact.is_some() {
            Evidence::OpenApi
        } else {
            Evidence::Recorded
        };
        LockCheckReport::compare(&existing, &current, evidence, self.versions.is_some())
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
}
