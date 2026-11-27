use crate::error::{IdlcError, IdlcResult};
use crate::fmt::format_idl_source;
use crate::library::LibraryLoader;
use crate::units::UnitGraph;
use similar::TextDiff;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use xidl_parser::hir::printer::IdlPrinter;
use xidl_parser::hir::{Definition, Pragma, Specification};
use xidl_parser::pipeline::Pipeline;
use xidl_parser::versioning::{VersionId, VersioningRecord};

/// A resolved contract and its declared history for human-readable comparison.
pub(crate) struct VersionDiff {
    spec: Specification,
    record: VersioningRecord,
    imports: String,
    imported_units: BTreeMap<String, String>,
    contract: String,
}

impl VersionDiff {
    /// Resolves imports and includes using the same preparation as generation.
    pub(crate) fn from_file(path: &Path) -> IdlcResult<Self> {
        let inputs = [path.to_path_buf()];
        let resolved = LibraryLoader::new().resolve(&inputs)?;
        resolved.check_compiler()?;
        let graph = UnitGraph::build(&inputs, resolved.mounts())?;
        let unit = graph
            .units()
            .last()
            .ok_or_else(|| IdlcError::fmt(format!("contract '{}' has no unit", path.display())))?;
        let mut resolver = crate::library::include_resolver(resolved.mounts());
        let spec = Pipeline::lower_spec(
            unit.typed_ast().clone(),
            &Default::default(),
            path,
            &mut *resolver,
        )?;
        let record = VersioningRecord::analyze(&spec)?.ok_or_else(|| {
            IdlcError::fmt(format!(
                "contract '{}' requires a @version enum",
                path.display()
            ))
        })?;
        let mut imports = String::new();
        for import in unit.imports() {
            imports.push_str(&format!(
                "import {};\n",
                serde_json::to_string(import.path())?
            ));
        }
        let imported_units = Self::collect_imported_units(&graph, &mut *resolver)?;
        Ok(Self {
            spec,
            record,
            imports,
            imported_units,
            contract: path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned(),
        })
    }

    /// Compares canonical projections; equal contracts return no output.
    pub(crate) fn compare(&self, from: &str, to: &str) -> IdlcResult<Option<String>> {
        let from_id = self.record.resolve_version(from)?;
        let to_id = self.record.resolve_version(to)?;
        let from_source = self.formatted_projection(from_id)?;
        let to_source = self.formatted_projection(to_id)?;
        let old_header = self.header(from_id)?;
        let new_header = self.header(to_id)?;
        let diff = TextDiff::from_lines(&from_source, &to_source)
            .unified_diff()
            .header(&old_header, &new_header)
            .to_string();
        Ok((!diff.is_empty()).then_some(diff))
    }

    /// Returns the analyzed versioning record for this contract.
    pub(crate) fn record(&self) -> &VersioningRecord {
        &self.record
    }

    /// Returns canonicalized IDL text for each transitively imported unit, keyed by unit path.
    pub(crate) fn imported_units(&self) -> &BTreeMap<String, String> {
        &self.imported_units
    }

    /// Formats a canonical projection for `version`.
    pub(crate) fn formatted_projection(&self, version: VersionId) -> IdlcResult<String> {
        let definitions = Self::unwrap_projected(self.record.project(&self.spec, version)?);
        let body = format_idl_source(&IdlPrinter::render(&Specification(definitions)))?;
        Ok(self.imports.clone() + &body)
    }

    fn unwrap_projected(projected: Specification) -> Vec<Definition> {
        // Projection's outer module and document version identify an artifact,
        // while the diff header or lock metadata already identifies each side.
        let mut definitions = Vec::new();
        for definition in projected.0 {
            match definition {
                Definition::Pragma(Pragma::XidlcOpenApiVersion(_)) => {}
                Definition::ModuleDcl(module) => definitions.extend(module.definition),
                definition => definitions.push(definition),
            }
        }
        definitions
    }

    fn collect_imported_units(
        graph: &UnitGraph,
        resolver: &mut dyn xidl_parser::parser::IncludeResolver,
    ) -> IdlcResult<BTreeMap<String, String>> {
        let units = graph.units();
        if units.len() <= 1 {
            return Ok(BTreeMap::new());
        }
        let mut bound_versions: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for unit in units {
            for import in unit.imports() {
                if let Some(version) = import.version() {
                    bound_versions
                        .entry(import.prefix())
                        .or_default()
                        .insert(version);
                }
            }
        }
        let mut canonical = BTreeMap::new();
        for dep in &units[..units.len() - 1] {
            let spec = Pipeline::lower_spec(
                dep.typed_ast().clone(),
                &Default::default(),
                dep.file(),
                resolver,
            )?;
            let rendered = match (
                VersioningRecord::analyze(&spec)?,
                bound_versions.get(dep.prefix()),
            ) {
                (Some(dep_record), Some(versions)) if !versions.is_empty() => {
                    let mut sections = String::new();
                    for raw_version in versions {
                        let version_id = dep_record.resolve_version(raw_version)?;
                        let defs = Self::unwrap_projected(dep_record.project(&spec, version_id)?);
                        let formatted =
                            format_idl_source(&IdlPrinter::render(&Specification(defs)))?;
                        sections.push_str(&format!("@version({raw_version})\n{formatted}"));
                    }
                    sections
                }
                _ => format_idl_source(&IdlPrinter::render(&spec))?,
            };
            canonical.insert(dep.path().to_string(), rendered);
        }
        Ok(canonical)
    }

    fn header(&self, version: VersionId) -> IdlcResult<String> {
        let definition = self.record.timeline().get(version).ok_or_else(|| {
            IdlcError::fmt(format!(
                "version diff: contract '{}' resolved an invalid version",
                self.contract
            ))
        })?;
        Ok(format!(
            "{} @ {} ({})",
            self.contract, definition.ident, definition.value
        ))
    }
}

#[cfg(test)]
mod tests;
