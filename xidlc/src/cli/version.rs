//! `xidlc version`: derived spine artifacts and the release-freeze gate.
//!
//! `manifest` prints the version spine (selector, timeline, per-version
//! wire-surface digests, adjacent-pair correspondence) of each contract.
//! `verify` compares the frozen identity of released versions — the wire
//! version selector and each version's surface digest — against a checked-in
//! lock file so an unannotated edit or a library upgrade that silently changes
//! an old version fails CI; `--update` is the explicit door for intentional
//! changes.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use serde_json::json;

use crate::error::{IdlcError, IdlcResult};

/// The lock file schema marker.
const LOCK_SCHEMA: &str = "xidl.versions-lock/v1";

/// `xidlc version manifest|verify`
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
        }
    }
}

fn manifest_for(file: &Path) -> IdlcResult<serde_json::Value> {
    let spine = spine_for(file)?;
    Ok(json!({
        "file": file.display().to_string(),
        "spine": spine,
    }))
}

fn spine_for(file: &Path) -> IdlcResult<Option<xidl_parser::versioning::Spine>> {
    let source = std::fs::read_to_string(file)
        .map_err(|error| IdlcError::fmt(format!("cannot read {}: {error}", file.display())))?;
    let typed = xidl_parser::parser::parser_text(&source)?;
    let spine = xidl_parser::hir::Specification::versioning_spine(typed)?;
    Ok(spine)
}

/// The frozen identity of one contract: the wire version selector (when the
/// contract declares one) plus every released version's surface digest.
fn contract_state(
    spine: &xidl_parser::versioning::Spine,
) -> serde_json::Map<String, serde_json::Value> {
    let mut state = serde_json::Map::new();
    state.insert(
        "selector".to_string(),
        match &spine.selector {
            Some(selector) => serde_json::Value::String(format!(
                "{}:{}",
                selector.location.as_str(),
                selector.name
            )),
            None => serde_json::Value::Null,
        },
    );
    state.insert(
        "versions".to_string(),
        serde_json::Value::Object(
            spine
                .surfaces
                .iter()
                .map(|surface| {
                    (
                        surface.version.ident.clone(),
                        serde_json::Value::String(surface.digest.clone()),
                    )
                })
                .collect(),
        ),
    );
    state
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
fn write_lock(
    lock_path: &Path,
    name: &str,
    current: &serde_json::Map<String, serde_json::Value>,
) -> IdlcResult<()> {
    let mut lock = read_lock(lock_path)?;
    lock["contracts"][name] = serde_json::Value::Object(current.clone());
    std::fs::write(lock_path, serde_json::to_string_pretty(&lock)? + "\n")?;
    println!("lock updated: {}", lock_path.display());
    Ok(())
}

/// Reads a lock file, tolerating absence but never corruption.
fn read_lock(lock_path: &Path) -> IdlcResult<serde_json::Value> {
    if !lock_path.exists() {
        return Ok(json!({
            "schema": LOCK_SCHEMA,
            "contracts": {},
        }));
    }
    let raw = std::fs::read_to_string(lock_path)
        .map_err(|error| IdlcError::fmt(format!("cannot read {}: {error}", lock_path.display())))?;
    let lock: serde_json::Value = serde_json::from_str(&raw)?;
    if lock.get("schema").and_then(serde_json::Value::as_str) != Some(LOCK_SCHEMA) {
        return Err(IdlcError::fmt(format!(
            "{} is not a {} lock file",
            lock_path.display(),
            LOCK_SCHEMA
        )));
    }
    Ok(lock)
}

/// Compares one contract's frozen identity against its current spine; returns
/// the drift report when the selector or a released version changed.
fn compare(
    lock_path: &Path,
    name: &str,
    current: &serde_json::Map<String, serde_json::Value>,
) -> IdlcResult<Option<String>> {
    let lock = read_lock(lock_path)?;
    let frozen = lock
        .get("contracts")
        .and_then(|contracts| contracts.get(name))
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut report = Vec::new();
    let frozen_selector = frozen.get("selector").unwrap_or(&serde_json::Value::Null);
    let current_selector = current.get("selector").unwrap_or(&serde_json::Value::Null);
    if frozen_selector != current_selector {
        report.push(format!(
            "version selector changed: {} → {}",
            describe_selector(frozen_selector),
            describe_selector(current_selector)
        ));
    }
    let frozen_versions = frozen
        .get("versions")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    let current_versions = current
        .get("versions")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (version, digest) in &frozen_versions {
        match current_versions.get(version) {
            Some(now) if now == digest => {}
            Some(now) => report.push(format!(
                "released version {version} changed: {} → {}",
                digest.as_str().unwrap_or_default(),
                now.as_str().unwrap_or_default()
            )),
            None => report.push(format!(
                "released version {version} missing from the contract ({})",
                digest.as_str().unwrap_or_default()
            )),
        }
    }
    for version in current_versions.keys() {
        if !frozen_versions.contains_key(version) {
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
fn describe_selector(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map(|selector| selector.to_string())
        .unwrap_or_else(|| "none".to_string())
}
