//! `xidlc version`: derived spine artifacts and the release-freeze gate.
//!
//! `manifest` prints the version spine (selector, timeline, per-version
//! wire-surface digests, adjacent-pair correspondence) of each contract.
//! `verify` compares the frozen identity of released versions — the wire
//! version selector, the timeline order and each version's surface digest and
//! wire value — against a checked-in lock file so an unannotated edit or a
//! library upgrade that silently changes an old version fails CI; `--update`
//! is the explicit door for intentional changes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{IdlcError, IdlcResult};

use xidl_parser::versioning::Spine;

/// The lock file schema marker.
const LOCK_SCHEMA: &str = "xidl.versions-lock/v1";

mod diff;

/// `xidlc version manifest|verify|diff`
#[derive(Debug, Args)]
pub struct ArgsVersion {
    #[command(subcommand)]
    pub(crate) command: VersionCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum VersionCommand {
    /// Emit the derived version spine JSON for each contract.
    Manifest(FilesArgs),
    /// Fail when a released version's wire surface drifted from the lock file.
    Verify(ArgsVerify),
    /// Review how one adjacent version pair carried over.
    Diff(ArgsDiff),
}

#[derive(Debug, Args)]
pub(crate) struct FilesArgs {
    files: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct ArgsVerify {
    /// Lock file holding the frozen digests; defaults to
    /// `<contract>.versions.lock.json` beside the contract.
    #[arg(long = "lock", value_name = "PATH")]
    pub(crate) lock: Option<PathBuf>,
    /// Rewrite the lock file with the current digests; the explicit door for
    /// intentional changes.
    #[arg(long = "update", default_value_t = false)]
    pub(crate) update: bool,
    pub(crate) files: Vec<PathBuf>,
}

/// How a requested version of a contract is named.
#[derive(Debug, Args)]
pub(crate) struct ArgsDiff {
    /// The older version of the pair: member ident or display value.
    /// Defaults to the second-to-last declared version.
    #[arg(long = "from", value_name = "VERSION")]
    pub(crate) from: Option<String>,
    /// The newer version of the pair: member ident or display value.
    /// Defaults to the last declared version.
    #[arg(long = "to", value_name = "VERSION")]
    pub(crate) to: Option<String>,
    /// Output format: review report or JSON.
    #[arg(long = "format", value_enum, default_value_t = DiffFormat::Text)]
    pub(crate) format: DiffFormat,
    pub(crate) files: Vec<PathBuf>,
}

/// The rendering of a version diff.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum DiffFormat {
    /// Human-readable review report.
    Text,
    /// Machine-readable JSON over the same correspondence cells.
    Json,
}

impl ArgsVersion {
    /// Runs the requested version subcommand.
    pub fn execute(self) -> IdlcResult<()> {
        match self.command {
            VersionCommand::Manifest(args) => {
                for file in &args.files {
                    println!("{}", serde_json::to_string_pretty(&manifest_for(file)?)?);
                }
                Ok(())
            }
            VersionCommand::Verify(args) => verify(&args),
            VersionCommand::Diff(args) => diff::render(&args),
        }
    }
}

fn manifest_for(file: &Path) -> IdlcResult<serde_json::Value> {
    let spine = spine_for(file)?;
    Ok(serde_json::json!({
        "file": file.display().to_string(),
        "spine": spine,
    }))
}

pub(super) fn spine_for(file: &Path) -> IdlcResult<Option<Spine>> {
    let source = std::fs::read_to_string(file)
        .map_err(|error| IdlcError::fmt(format!("cannot read {}: {error}", file.display())))?;
    let typed = xidl_parser::parser::parser_text(&source)?;
    let spine = xidl_parser::hir::Specification::versioning_spine(typed)?;
    Ok(spine)
}

/// One frozen version: the canonical surface digest plus the wire value the
/// selector carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrozenVersion {
    digest: String,
    value: String,
}

/// One contract's frozen identity: the wire version selector, the declared
/// timeline order and every released version's frozen facts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FrozenContract {
    #[serde(default)]
    selector: Value,
    #[serde(default)]
    timeline: Vec<String>,
    #[serde(default)]
    versions: BTreeMap<String, FrozenVersion>,
}

/// The checked-in lock file: every contract's frozen identity.
#[derive(Debug, Serialize, Deserialize)]
struct LockFile {
    schema: String,
    #[serde(default)]
    contracts: BTreeMap<String, FrozenContract>,
}

/// The frozen identity of one contract: the wire version selector (when the
/// contract declares one), the timeline order and each released version's
/// surface digest and wire value.
fn contract_state(spine: &Spine) -> FrozenContract {
    FrozenContract {
        selector: match &spine.selector {
            Some(selector) => {
                Value::String(format!("{}:{}", selector.location.as_str(), selector.name))
            }
            None => Value::Null,
        },
        timeline: spine.timeline.iter().map(|def| def.ident.clone()).collect(),
        versions: spine
            .surfaces
            .iter()
            .map(|surface| {
                (
                    surface.version.ident.clone(),
                    FrozenVersion {
                        digest: surface.digest.clone(),
                        value: surface.version.value.clone(),
                    },
                )
            })
            .collect(),
    }
}

fn verify(args: &ArgsVerify) -> IdlcResult<()> {
    let mut drifted = Vec::new();
    for file in &args.files {
        let lock_path = match &args.lock {
            Some(path) => path.clone(),
            None => lock_path_for(file),
        };
        let Some(spine) = spine_for(file)? else {
            continue;
        };
        let name = contract_name(file);
        let current = contract_state(&spine);
        if args.update {
            write_lock(&lock_path, &name, &current)?;
            continue;
        }
        if !lock_path.exists() {
            return Err(IdlcError::fmt(format!(
                "no lock file {}; create it with `xidlc version verify --update`",
                lock_path.display()
            )));
        }
        if let Some(report) = compare(&lock_path, &name, &current)? {
            drifted.push(report);
        }
    }
    if drifted.is_empty() {
        Ok(())
    } else {
        Err(IdlcError::fmt(drifted.join("\n")))
    }
}

fn contract_name(file: &Path) -> String {
    file.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("contract")
        .to_string()
}

fn lock_path_for(file: &Path) -> PathBuf {
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("contract");
    let mut lock = file.to_path_buf();
    lock.set_file_name(format!("{stem}.versions.lock.json"));
    lock
}

/// Rewrites one contract's entry inside the lock file, preserving every
/// other contract's frozen identity.
fn write_lock(lock_path: &Path, name: &str, current: &FrozenContract) -> IdlcResult<()> {
    let mut lock = read_lock(lock_path)?;
    lock.contracts.insert(name.to_string(), current.clone());
    std::fs::write(lock_path, serde_json::to_string_pretty(&lock)? + "\n")?;
    println!("lock updated: {}", lock_path.display());
    Ok(())
}

/// Reads a lock file, tolerating absence but never corruption: a file that
/// does not parse as the lock schema is a typed error for both reading and
/// rewriting, never a silent empty slate.
fn read_lock(lock_path: &Path) -> IdlcResult<LockFile> {
    if !lock_path.exists() {
        return Ok(LockFile {
            schema: LOCK_SCHEMA.to_string(),
            contracts: BTreeMap::new(),
        });
    }
    let raw = std::fs::read_to_string(lock_path)
        .map_err(|error| IdlcError::fmt(format!("cannot read {}: {error}", lock_path.display())))?;
    let lock: LockFile = serde_json::from_str(&raw).map_err(|error| {
        IdlcError::fmt(format!(
            "{} does not parse as a {} lock file: {error}",
            lock_path.display(),
            LOCK_SCHEMA
        ))
    })?;
    if lock.schema != LOCK_SCHEMA {
        return Err(IdlcError::fmt(format!(
            "{} is not a {} lock file",
            lock_path.display(),
            LOCK_SCHEMA
        )));
    }
    Ok(lock)
}

/// Compares one contract's frozen identity against its current spine; returns
/// the drift report when the selector, the timeline, a version's wire value
/// or a version's surface digest changed.
fn compare(lock_path: &Path, name: &str, current: &FrozenContract) -> IdlcResult<Option<String>> {
    let lock = read_lock(lock_path)?;
    let frozen = lock.contracts.get(name).cloned().unwrap_or_default();
    let mut report = Vec::new();
    if frozen.selector != current.selector {
        report.push(format!(
            "version selector changed: {} → {}",
            describe_selector(&frozen.selector),
            describe_selector(&current.selector)
        ));
    }
    if frozen.timeline != current.timeline {
        report.push(format!(
            "version timeline changed: {} → {}",
            frozen.timeline.join(", "),
            current.timeline.join(", ")
        ));
    }
    for (version, was) in &frozen.versions {
        match current.versions.get(version) {
            Some(now) => {
                if now.digest != was.digest {
                    report.push(format!(
                        "released version {version} changed: {} → {}",
                        was.digest, now.digest
                    ));
                }
                if now.value != was.value {
                    report.push(format!(
                        "released version {version} changed its wire value: {} → {}",
                        was.value, now.value
                    ));
                }
            }
            None => report.push(format!(
                "released version {version} missing from the contract ({})",
                was.digest
            )),
        }
    }
    for version in current.versions.keys() {
        if !frozen.versions.contains_key(version) {
            report.push(format!("new version {version} is not frozen yet"));
        }
    }
    if report.is_empty() {
        Ok(None)
    } else {
        Ok(Some(format!(
            "release freeze violated ({}):\n{}",
            lock_path.display(),
            report.join("\n")
        )))
    }
}

/// Renders a frozen selector value for drift reports.
fn describe_selector(value: &Value) -> String {
    value
        .as_str()
        .map(|selector| selector.to_string())
        .unwrap_or_else(|| "none".to_string())
}
