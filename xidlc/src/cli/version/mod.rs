//! `xidlc version`: the release-freeze gate over normalized OpenAPI copies.
//!
//! Every version's normalized OpenAPI document is checked in under
//! `<contract>.versions/<value>.json`. `verify` compares the current output
//! of each released version against its checked-in copy, so an unannotated
//! edit that silently changes a released version fails CI; `--update` is the
//! explicit door for intentional changes and refuses to bless history
//! rewrites: released versions may only disappear by deletion and new
//! versions may only join at the end of the timeline. `diff` prints the
//! structural difference between two versions' documents.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use serde_json::Value;

use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir::ParserProperties;
use xidl_parser::versioning::VersionDef;

mod diff;
mod normalize;

/// The release-order manifest schema marker.
const INDEX_SCHEMA: &str = "xidl.versions-index/v1";

/// `xidlc version verify|diff`
#[derive(Debug, Args)]
pub struct ArgsVersion {
    #[command(subcommand)]
    pub(crate) command: VersionCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum VersionCommand {
    /// Fail when a released version's normalized OpenAPI drifted from its
    /// checked-in copy.
    Verify(ArgsVerify),
    /// Print the structural difference between two versions' documents.
    Diff(ArgsDiff),
}

#[derive(Debug, Args)]
pub(crate) struct ArgsVerify {
    /// Rewrite the checked-in copies with the current output; the explicit
    /// door for intentional changes.
    #[arg(long = "update", default_value_t = false)]
    pub(crate) update: bool,
    pub(crate) files: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct ArgsDiff {
    /// The older version: member ident or display value.
    #[arg(long = "from", value_name = "VERSION")]
    pub(crate) from: Option<String>,
    /// The newer version: member ident or display value.
    #[arg(long = "to", value_name = "VERSION")]
    pub(crate) to: Option<String>,
    pub(crate) files: Vec<PathBuf>,
}

impl ArgsVersion {
    /// Runs the requested version subcommand.
    pub fn execute(self) -> IdlcResult<()> {
        match self.command {
            VersionCommand::Verify(args) => verify(&args),
            VersionCommand::Diff(args) => diff::render(&args),
        }
    }
}

/// One version's normalized document and its timeline identity.
struct FrozenVersion {
    definition: VersionDef,
    document: Value,
}

/// One contract's current per-version output, in timeline order.
struct ContractOutput {
    versions: Vec<FrozenVersion>,
}

/// Verifies or rewrites the checked-in copies of every contract.
fn verify(args: &ArgsVerify) -> IdlcResult<()> {
    let mut reports = Vec::new();
    for file in &args.files {
        let output = contract_output(file)?;
        if args.update {
            update(file, &output)?;
        } else if let Some(report) = compare(file, &output)? {
            reports.push(report);
        }
    }
    if reports.is_empty() {
        Ok(())
    } else {
        Err(IdlcError::fmt(reports.join("\n")))
    }
}

/// Renders every version of one contract as a normalized OpenAPI document.
fn contract_output(file: &Path) -> IdlcResult<ContractOutput> {
    let source = std::fs::read_to_string(file)
        .map_err(|error| IdlcError::fmt(format!("cannot read {}: {error}", file.display())))?;
    let typed = xidl_parser::parser::parser_text(&source)?;
    let mut properties = ParserProperties::default();
    properties.insert(
        "hir_kind".to_string(),
        serde_json::Value::String("http".to_string()),
    );
    properties.insert(
        xidl_parser::versioning::VERSION_LAYOUT_PROPERTY.to_string(),
        serde_json::Value::String(xidl_parser::versioning::LAYOUT_PER_VERSION.to_string()),
    );
    let projections =
        xidl_parser::hir::Specification::project_typed_ast_versions(typed, properties, file)?;
    let mut versions = Vec::new();
    for projection in projections {
        let Some(definition) = projection.version else {
            continue;
        };
        let xidl_parser::rest_hir::ProjectedHir::Http(rest_hir) = projection.hir else {
            continue;
        };
        let mut document =
            crate::generate::openapi::render_openapi_json(&rest_hir.spec, &rest_hir)?;
        normalize::normalize_document(&mut document);
        versions.push(FrozenVersion {
            definition,
            document,
        });
    }
    if versions.is_empty() {
        return Err(IdlcError::fmt(format!(
            "{} declares no @version enum; `xidlc version` freezes versioned contracts",
            file.display()
        )));
    }
    Ok(ContractOutput { versions })
}

/// The checked-in copies directory of one contract.
fn freeze_dir(file: &Path) -> PathBuf {
    let stem = contract_name(file);
    let mut dir = file.to_path_buf();
    dir.set_file_name(format!("{stem}.versions"));
    dir
}

fn contract_name(file: &Path) -> String {
    file.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("contract")
        .to_string()
}

/// The released version values, in release order.
fn read_index(dir: &Path) -> IdlcResult<Vec<String>> {
    let path = dir.join("index.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| IdlcError::fmt(format!("cannot read {}: {error}", path.display())))?;
    let index: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
        IdlcError::fmt(format!(
            "{} does not parse as an {INDEX_SCHEMA} manifest: {error}",
            path.display()
        ))
    })?;
    if index.get("schema").and_then(Value::as_str) != Some(INDEX_SCHEMA) {
        return Err(IdlcError::fmt(format!(
            "{} is not an {INDEX_SCHEMA} manifest",
            path.display()
        )));
    }
    Ok(index
        .get("released")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

/// Rewrites the release-order manifest.
fn write_index(dir: &Path, released: &[String]) -> IdlcResult<()> {
    let index = serde_json::json!({
        "schema": INDEX_SCHEMA,
        "released": released,
    });
    std::fs::write(
        dir.join("index.json"),
        serde_json::to_string_pretty(&index)? + "\n",
    )?;
    Ok(())
}

/// Compares one contract's current output against its checked-in copies.
fn compare(file: &Path, output: &ContractOutput) -> IdlcResult<Option<String>> {
    let dir = freeze_dir(file);
    let released = read_index(&dir)?;
    if released.is_empty() {
        return Err(IdlcError::fmt(format!(
            "no frozen versions under {}; create them with `xidlc version verify --update`",
            dir.display()
        )));
    }
    let mut report: Vec<String> = Vec::new();
    for value in &released {
        let Some(current) = output
            .versions
            .iter()
            .find(|version| &version.definition.value == value)
        else {
            report.push(format!(
                "released version {value} disappeared from the contract"
            ));
            continue;
        };
        let frozen = dir.join(format!("{}.json", value));
        let Ok(previous) = std::fs::read_to_string(&frozen) else {
            report.push(format!("released version {value} lost its checked-in copy"));
            continue;
        };
        let now = serde_json::to_string_pretty(&current.document)? + "\n";
        if now != previous {
            let old: Value = serde_json::from_str(&previous).unwrap_or(Value::Null);
            report.push(format!(
                "released version {value} changed:\n{}",
                diff::structural(&old, &current.document)
            ));
        }
    }
    for version in &output.versions {
        if !released.contains(&version.definition.value) {
            report.push(format!(
                "new version {} ({}) is not frozen yet; run `xidlc version verify --update` to bless it",
                version.definition.ident, version.definition.value
            ));
        }
    }
    if report.is_empty() {
        Ok(None)
    } else {
        Ok(Some(format!(
            "release freeze violated ({}):\n{}",
            dir.display(),
            report.join("\n")
        )))
    }
}

/// Rewrites every checked-in copy and the release-order manifest.
fn update(file: &Path, output: &ContractOutput) -> IdlcResult<()> {
    let dir = freeze_dir(file);
    let released = read_index(&dir)?;
    let position = |value: &str| {
        output
            .versions
            .iter()
            .position(|version| version.definition.value == value)
    };
    for new in &output.versions {
        if released.contains(&new.definition.value) {
            continue;
        }
        let first_released = released.iter().filter_map(|value| position(value)).min();
        if let Some(index) = first_released {
            if position(&new.definition.value).is_some_and(|own| own < index) {
                return Err(IdlcError::fmt(format!(
                    "new version {} ({}) is inserted before released version {}; released history is append-only",
                    new.definition.ident,
                    new.definition.value,
                    released.join(", ")
                )));
            }
        }
    }
    let mut order = released.clone();
    for version in &output.versions {
        if !order.contains(&version.definition.value) {
            order.push(version.definition.value.clone());
        }
    }
    let positions: Vec<Option<usize>> = released.iter().map(|value| position(value)).collect();
    let unordered = positions
        .iter()
        .zip(positions.iter().skip(1))
        .any(|(first, second)| match (first, second) {
            (Some(first), Some(second)) => first >= second,
            _ => false,
        });
    if unordered {
        return Err(IdlcError::fmt(format!(
            "released versions {} were reordered; released history is append-only",
            released.join(", ")
        )));
    }
    std::fs::create_dir_all(&dir)?;
    for version in &output.versions {
        let content = serde_json::to_string_pretty(&version.document)? + "\n";
        std::fs::write(
            dir.join(format!("{}.json", version.definition.value)),
            content,
        )?;
    }
    write_index(&dir, &order)?;
    println!("versions updated: {}", dir.display());
    Ok(())
}
