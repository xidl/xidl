//! `xidlc version diff`: the structural difference between two versions.
//!
//! The comparison runs over normalized documents, so prose cannot shadow a
//! wire change: only paths, parameters, schemas and statuses that really
//! differ show up.

use std::path::Path;

use serde_json::Value;

use super::{ArgsDiff, ContractOutput, FrozenVersion, contract_output};
use crate::error::{IdlcError, IdlcResult};

/// Prints the structural difference between two versions of each contract.
pub(crate) fn render(args: &ArgsDiff) -> IdlcResult<()> {
    for file in &args.files {
        let output = contract_output(file)?;
        let (older, newer) = select_pair(file, &output, args.from.as_deref(), args.to.as_deref())?;
        println!(
            "{}: {} → {}",
            file.display(),
            older.definition.value,
            newer.definition.value
        );
        let report = structural(&older.document, &newer.document);
        if report.is_empty() {
            println!("  no structural difference");
        } else {
            print!("{report}");
        }
    }
    Ok(())
}

/// Resolves the version pair, defaulting to the newest adjacent pair.
fn select_pair<'a>(
    file: &Path,
    output: &'a ContractOutput,
    from: Option<&str>,
    to: Option<&str>,
) -> IdlcResult<(&'a FrozenVersion, &'a FrozenVersion)> {
    let resolve = |needle: &str| {
        output
            .versions
            .iter()
            .find(|version| {
                version.definition.ident == needle || version.definition.value == needle
            })
            .ok_or_else(|| {
                IdlcError::fmt(format!(
                    "{} declares no version '{}' (declared: {})",
                    file.display(),
                    needle,
                    output
                        .versions
                        .iter()
                        .map(|version| version.definition.ident.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    };
    let newer = match to {
        Some(needle) => resolve(needle)?,
        None => output
            .versions
            .last()
            .expect("versioned contracts declare at least one version"),
    };
    let older = match from {
        Some(needle) => resolve(needle)?,
        None => {
            let index = output
                .versions
                .iter()
                .position(|version| std::ptr::eq(version, newer))
                .expect("newer comes from the same output");
            let previous = index
                .checked_sub(1)
                .and_then(|index| output.versions.get(index));
            previous.ok_or_else(|| {
                IdlcError::fmt(format!(
                    "{} has no version before {}",
                    file.display(),
                    newer.definition.value
                ))
            })?
        }
    };
    if std::ptr::eq(older, newer) {
        return Err(IdlcError::fmt(format!(
            "{}: diff needs two distinct versions",
            file.display()
        )));
    }
    Ok((older, newer))
}

/// Renders the structural difference between two documents as line edits.
pub(crate) fn structural(old: &Value, new: &Value) -> String {
    let mut lines = Vec::new();
    walk(old, new, String::new(), &mut lines);
    lines.join("\n") + "\n"
}

fn walk(old: &Value, new: &Value, path: String, lines: &mut Vec<String>) {
    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            for (key, was) in old_map {
                let deeper = format!("{path}/{key}");
                match new_map.get(key) {
                    Some(now) => walk(was, now, deeper, lines),
                    None => lines.push(format!("- {deeper}: {}", describe(was))),
                }
            }
            for (key, now) in new_map {
                if !old_map.contains_key(key) {
                    lines.push(format!("+ {path}/{key}: {}", describe(now)));
                }
            }
        }
        (Value::Array(old_items), Value::Array(new_items)) => {
            for (index, was) in old_items.iter().enumerate() {
                let deeper = format!("{path}/{index}");
                match new_items.get(index) {
                    Some(now) => walk(was, now, deeper, lines),
                    None => lines.push(format!("- {deeper}: {}", describe(was))),
                }
            }
            for (index, now) in new_items.iter().enumerate().skip(old_items.len()) {
                lines.push(format!("+ {path}/{index}: {}", describe(now)));
            }
        }
        _ => {
            if old != new {
                lines.push(format!("~ {path}: {} → {}", describe(old), describe(new)));
            }
        }
    }
}

fn describe(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}
