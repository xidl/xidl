//! `xidlc version`: derived spine artifacts and the release-freeze gate.
//!
//! `manifest` prints the version spine (selector, timeline, per-version
//! wire-surface digests, adjacent-pair correspondence) of each contract.
//! `diff` renders one adjacent version pair as a review report. `verify`
//! compares the frozen identity of released versions — the wire version
//! selector and each version's surface digest — against a checked-in lock
//! file so an unannotated edit or a library upgrade that silently changes an
//! old version fails CI; `--update` is the explicit door for intentional
//! changes.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum};
use serde_json::json;

use crate::error::{IdlcError, IdlcResult};

use xidl_parser::versioning::{CellKind, CorrespondenceCell, Evidence, Spine, VersionDef};

/// The lock file schema marker.
const LOCK_SCHEMA: &str = "xidl.versions-lock/v1";

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
            VersionCommand::Diff(args) => diff(&args),
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

/// Renders one adjacent version pair as a review report or JSON.
fn diff(args: &ArgsDiff) -> IdlcResult<()> {
    for file in &args.files {
        let Some(spine) = spine_for(file)? else {
            continue;
        };
        let (from, to) = select_pair(&spine, args.from.as_deref(), args.to.as_deref())?;
        let cells: Vec<&CorrespondenceCell> = spine
            .correspondence
            .iter()
            .filter(|cell| cell.from == from.ident && cell.to == to.ident)
            .collect();
        match args.format {
            DiffFormat::Json => {
                println!("{}", diff_json(&spine, from, to, &cells)?);
            }
            DiffFormat::Text => {
                print!("{}", diff_text(from, to, spine.selector.as_ref(), &cells));
            }
        }
    }
    Ok(())
}

/// Resolves the requested adjacent pair, defaulting to the newest pair.
fn select_pair<'a>(
    spine: &'a Spine,
    from: Option<&str>,
    to: Option<&str>,
) -> IdlcResult<(&'a VersionDef, &'a VersionDef)> {
    if spine.timeline.len() < 2 {
        return Err(IdlcError::fmt(
            "a version diff needs at least two declared versions",
        ));
    }
    let from_def = match from {
        Some(name) => resolve_version(&spine.timeline, name)?,
        None => &spine.timeline[spine.timeline.len() - 2],
    };
    let to_def = match to {
        Some(name) => resolve_version(&spine.timeline, name)?,
        None => spine
            .timeline
            .last()
            .expect("checked at least two versions"),
    };
    if from_def.ident == to_def.ident {
        return Err(IdlcError::fmt(format!(
            "a version diff needs two different versions; {} names one version twice",
            from_def.ident
        )));
    }
    let position = |def: &VersionDef| {
        spine
            .timeline
            .iter()
            .position(|entry| entry.ident == def.ident)
            .expect("the version comes from this timeline")
    };
    match position(to_def).cmp(&position(from_def)) {
        std::cmp::Ordering::Less => Err(IdlcError::fmt(format!(
            "version diff runs forward in the timeline: {} is declared before {}",
            to_def.ident, from_def.ident
        ))),
        std::cmp::Ordering::Equal => unreachable!("identical versions rejected above"),
        std::cmp::Ordering::Greater if position(to_def) == position(from_def) + 1 => {
            Ok((from_def, to_def))
        }
        std::cmp::Ordering::Greater => Err(IdlcError::fmt(format!(
            "the spine records adjacent version pairs only: {} → {} skips versions; diff each adjacent pair",
            from_def.ident, to_def.ident
        ))),
    }
}

/// Resolves one version by member ident or display value.
fn resolve_version<'a>(timeline: &'a [VersionDef], name: &str) -> IdlcResult<&'a VersionDef> {
    let mut matches = timeline
        .iter()
        .filter(|def| def.ident == name || def.value == name);
    let Some(first) = matches.next() else {
        let declared = timeline
            .iter()
            .map(|def| def.ident.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(IdlcError::fmt(format!(
            "version {name} is not declared (declared: {declared})"
        )));
    };
    if matches.next().is_some() {
        return Err(IdlcError::fmt(format!(
            "version {name} matches both a member ident and another version's display value; use the member ident"
        )));
    }
    Ok(first)
}

/// JSON rendering: the pair's identity plus its correspondence cells.
fn diff_json(
    spine: &Spine,
    from: &VersionDef,
    to: &VersionDef,
    cells: &[&CorrespondenceCell],
) -> IdlcResult<String> {
    Ok(serde_json::to_string_pretty(&json!({
        "from": version_identity(spine, from),
        "to": version_identity(spine, to),
        "selector": spine.selector,
        "cells": cells,
        "summary": diff_summary(cells),
    }))?)
}

/// Text rendering: one line per non-mechanical cell, mechanical as a count.
fn diff_text(
    from: &VersionDef,
    to: &VersionDef,
    selector: Option<&xidl_parser::versioning::VersionSelector>,
    cells: &[&CorrespondenceCell],
) -> String {
    let mut out = format!(
        "diff {} ({}) → {} ({})\n",
        from.value, from.ident, to.value, to.ident
    );
    match selector {
        Some(selector) => out.push_str(&format!(
            "selector: {} \"{}\"\n",
            selector.location.as_str(),
            selector.name
        )),
        None => out.push_str("selector: none\n"),
    }
    for cell in cells
        .iter()
        .filter(|cell| cell.kind != CellKind::Mechanical)
    {
        let from_subject = cell.from_subject.as_deref().unwrap_or("-");
        let to_subject = cell.to_subject.as_deref().unwrap_or("-");
        out.push_str(&format!(
            "  {:<13} {} → {}  [{}]\n",
            kind_label(cell.kind),
            from_subject,
            to_subject,
            evidence_label(&cell.evidence),
        ));
        if is_review_cell(cell) {
            out.push_str("              ^ review before adapters rely on it\n");
        }
    }
    let mechanical = cells
        .iter()
        .filter(|cell| cell.kind == CellKind::Mechanical)
        .count();
    out.push_str(&format!("mechanical cells: {mechanical}\n"));
    out.push_str(&format!("summary: {}\n", summary_line(cells)));
    out
}

/// The version's identity: member ident, wire value, frozen-relevant digest.
fn version_identity(spine: &Spine, def: &VersionDef) -> serde_json::Value {
    let digest = spine
        .surfaces
        .iter()
        .find(|surface| surface.version.ident == def.ident)
        .map(|surface| serde_json::Value::String(surface.digest.clone()))
        .unwrap_or(serde_json::Value::Null);
    json!({"ident": def.ident, "value": def.value, "digest": digest})
}

/// Counts the cells by kind plus the review-side total.
fn diff_summary(cells: &[&CorrespondenceCell]) -> serde_json::Value {
    let mut counts: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for cell in cells {
        *counts
            .entry(kind_label(cell.kind).to_string())
            .or_insert(0u64) += 1;
    }
    counts.insert(
        "review".to_string(),
        cells.iter().filter(|cell| is_review_cell(cell)).count() as u64,
    );
    serde_json::to_value(counts).expect("a map of counts serializes")
}

/// One-line text summary next to the JSON summary's numbers.
fn summary_line(cells: &[&CorrespondenceCell]) -> String {
    let mut parts = Vec::new();
    for kind in [
        CellKind::Added,
        CellKind::Removed,
        CellKind::Renamed,
        CellKind::TypeChanged,
        CellKind::Optionalized,
        CellKind::NameShifted,
    ] {
        let count = cells.iter().filter(|cell| cell.kind == kind).count();
        if count > 0 {
            parts.push(format!("{count} {}", kind_label(kind)));
        }
    }
    let review = cells.iter().filter(|cell| is_review_cell(cell)).count();
    parts.push(format!("{review} review"));
    parts.join(", ")
}

/// Whether a cell needs human review before tooling may rely on it: a wire
/// coincidence, or an added fact whose resume evidence is not rename-backed.
fn is_review_cell(cell: &CorrespondenceCell) -> bool {
    matches!(cell.kind, CellKind::NameShifted)
        || (matches!(cell.kind, CellKind::Added)
            && matches!(
                cell.evidence,
                Evidence::ResumesAfterGap {
                    confirmed: false,
                    ..
                }
            ))
}

/// The kebab-case label of a cell kind, matching the JSON rendering.
fn kind_label(kind: CellKind) -> &'static str {
    match kind {
        CellKind::Mechanical => "mechanical",
        CellKind::Renamed => "renamed",
        CellKind::NameShifted => "name-shifted",
        CellKind::TypeChanged => "type-changed",
        CellKind::Optionalized => "optionalized",
        CellKind::Added => "added",
        CellKind::Removed => "removed",
    }
}

/// A short human label of a cell's audit evidence.
fn evidence_label(evidence: &Evidence) -> String {
    match evidence {
        Evidence::Same => "unchanged".to_string(),
        Evidence::ExplicitRename => "explicit-rename".to_string(),
        Evidence::WireNameMatch => "wire-name-match".to_string(),
        Evidence::ResumesAfterGap { gap, confirmed } => format!(
            "resume after {gap}, {}",
            if *confirmed {
                "confirmed"
            } else {
                "unconfirmed"
            }
        ),
        Evidence::IntervalStart => "interval-start".to_string(),
        Evidence::IntervalEnd => "interval-end".to_string(),
    }
}
