//! `xidlc version`: derived spine artifacts and the release-freeze gate.
//!
//! `manifest` prints the version spine (timeline, per-version wire-surface
//! digests, adjacent-pair correspondence) of each contract. `verify` compares
//! the digests of released versions against a checked-in lock file so an
//! unannotated edit or a library upgrade that silently changes an old version
//! fails CI; `--update` is the explicit door for intentional changes.

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

fn digests(spine: &xidl_parser::versioning::Spine) -> serde_json::Map<String, serde_json::Value> {
    spine
        .surfaces
        .iter()
        .map(|surface| {
            (
                surface.version.ident.clone(),
                serde_json::Value::String(surface.digest.clone()),
            )
        })
        .collect()
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
        let current = digests(&spine);
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
/// other contract's frozen digests.
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

/// Compares frozen digests against the current spine; returns the drift
/// report when released versions changed.
/// Compares one contract's frozen digests against its current spine; returns
/// the drift report when a released version changed.
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
    for (version, digest) in &frozen {
        match current.get(version) {
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
    for version in current.keys() {
        if !frozen.contains_key(version) {
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
