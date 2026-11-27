//! Drift diagnostics for version lock and check.

use super::{LockDocument, VersionArtifact};
use crate::error::{IdlcError, IdlcResult};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Evidence {
    OpenApi,
    Recorded,
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
        artifact: Option<VersionArtifact>,
    },
    /// A locked version was removed from the contract.
    Removed { version: String, id: String },
    /// The selected version is absent from the lock file.
    Unlocked { version: String, id: String },
    /// A locked version has no stored OpenAPI fingerprint.
    OpenApiUnlocked { version: String, id: String },
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
                artifact,
            } => {
                let subject = match artifact {
                    Some(VersionArtifact::OpenApi) => " OpenAPI artifact",
                    None => "",
                };
                format!(
                    "version '{version}' ({id}){subject} changed: locked {expected}, current {actual}"
                )
            }
            Self::Removed { version, id } => {
                format!("version '{version}' ({id}) was removed from the contract")
            }
            Self::Unlocked { version, id } => {
                format!("version '{version}' ({id}) is not present in the lock file")
            }
            Self::OpenApiUnlocked { version, id } => format!(
                "version '{version}' ({id}) has no OpenAPI artifact fingerprint; run version lock --artifact openapi"
            ),
        }
    }
}

/// Outcome of validating a contract against its version lock file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LockCheckReport {
    pub(crate) drifts: Vec<LockDrift>,
}

impl LockCheckReport {
    pub(super) fn compare(
        existing: &LockDocument,
        current: &LockDocument,
        evidence: Evidence,
        require_selected: bool,
    ) -> IdlcResult<Self> {
        let mut drifts = Vec::new();
        for (version, locked) in &existing.versions {
            let Some(actual) = current.versions.get(version) else {
                drifts.push(LockDrift::Removed {
                    version: version.clone(),
                    id: locked.id.clone(),
                });
                continue;
            };
            if evidence != Evidence::OpenApi && locked.fingerprint != actual.fingerprint {
                drifts.push(LockDrift::Changed {
                    version: version.clone(),
                    id: actual.id.clone(),
                    expected: locked.fingerprint.clone(),
                    actual: actual.fingerprint.clone(),
                    artifact: None,
                });
            }
            match (&locked.openapi_fingerprint, &actual.openapi_fingerprint) {
                (None, _) if evidence == Evidence::OpenApi => {
                    drifts.push(LockDrift::OpenApiUnlocked {
                        version: version.clone(),
                        id: actual.id.clone(),
                    });
                }
                (Some(expected), Some(found)) if expected != found => {
                    drifts.push(LockDrift::Changed {
                        version: version.clone(),
                        id: actual.id.clone(),
                        expected: expected.clone(),
                        actual: found.clone(),
                        artifact: Some(VersionArtifact::OpenApi),
                    });
                }
                (Some(_), None) => {
                    return Err(IdlcError::fmt(format!(
                        "version lock: OpenAPI fingerprint was not generated for version '{version}'"
                    )));
                }
                _ => {}
            }
        }
        if require_selected {
            for (version, actual) in &current.versions {
                if !existing.versions.contains_key(version) {
                    drifts.push(LockDrift::Unlocked {
                        version: version.clone(),
                        id: actual.id.clone(),
                    });
                }
            }
        }
        Ok(Self { drifts })
    }

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
