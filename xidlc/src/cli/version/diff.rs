//! The diff view: one adjacent version pair rendered as a review report or
//! machine-readable JSON over the same correspondence cells.

use serde_json::json;

use xidl_parser::versioning::{CellKind, CorrespondenceCell, Evidence, Spine, VersionDef};

use crate::error::{IdlcError, IdlcResult};

use super::{ArgsDiff, DiffFormat, spine_for};

/// Renders one adjacent version pair as a review report or JSON.
pub(super) fn render(args: &ArgsDiff) -> IdlcResult<()> {
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
