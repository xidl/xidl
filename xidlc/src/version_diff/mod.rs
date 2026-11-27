use crate::error::{IdlcError, IdlcResult};
use crate::fmt::format_idl_source;
use crate::library::{LibraryLoader, ResolvedInputs};
use crate::units::{Unit, UnitGraph};
use similar::TextDiff;
use std::path::Path;
use xidl_parser::hir::printer::IdlPrinter;
use xidl_parser::hir::{Definition, ParserProperties, Pragma, Specification};
use xidl_parser::pipeline::Pipeline;
use xidl_parser::semantic::INTEGER_IMPORTS_PROPERTY;
use xidl_parser::versioning::{VersionId, VersioningRecord};

/// A resolved contract and its declared history for human-readable comparison.
pub(crate) struct VersionDiff {
    spec: Specification,
    record: VersioningRecord,
    imports: String,
    contract: String,
}

impl VersionDiff {
    /// Resolves imports and includes using the same preparation as generation.
    pub(crate) fn from_file(path: &Path) -> IdlcResult<Self> {
        Self::load(path).map(|(contract, _, _)| contract)
    }

    /// Prepares a single contract with its resolved dependencies and mounts.
    pub(crate) fn load(path: &Path) -> IdlcResult<(Self, UnitGraph, ResolvedInputs)> {
        let inputs = [path.to_path_buf()];
        let resolved = LibraryLoader::new().resolve(&inputs)?;
        resolved.check_compiler()?;
        let [input] = resolved.files() else {
            return Err(IdlcError::fmt(format!(
                "contract '{}' must resolve to one IDL input",
                path.display()
            )));
        };
        let graph = UnitGraph::build(resolved.files(), resolved.mounts())?;
        let unit = graph
            .units()
            .last()
            .ok_or_else(|| IdlcError::fmt(format!("contract '{}' has no unit", path.display())))?;
        let mut resolver = crate::library::include_resolver(resolved.mounts());
        let spec = Self::lower_unit(&graph, unit, &mut *resolver)?;
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
        let diff = Self {
            spec,
            record,
            imports,
            contract: input
                .file_name()
                .unwrap_or(input.as_os_str())
                .to_string_lossy()
                .into_owned(),
        };
        Ok((diff, graph, resolved))
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

    /// Formats a canonical projection for `version`.
    pub(crate) fn formatted_projection(&self, version: VersionId) -> IdlcResult<String> {
        self.format_projection(self.project(version)?)
    }

    /// Builds the declarations retained in a canonical projection.
    pub(crate) fn project(&self, version: VersionId) -> IdlcResult<Vec<Definition>> {
        Ok(Self::unwrap_projected(
            self.record.project(&self.spec, version)?,
        ))
    }

    /// Formats already-projected declarations with their import statements.
    pub(crate) fn format_projection(&self, definitions: Vec<Definition>) -> IdlcResult<String> {
        let body = format_idl_source(&IdlPrinter::render(&Specification(definitions)))?;
        Ok(self.imports.clone() + &body)
    }

    /// Removes the artifact wrapper around a projected contract.
    pub(crate) fn unwrap_projected(projected: Specification) -> Vec<Definition> {
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

    /// Lowers a unit with its resolved integer declarations and includes.
    pub(crate) fn lower_unit(
        graph: &UnitGraph,
        unit: &Unit,
        resolver: &mut dyn xidl_parser::parser::IncludeResolver,
    ) -> IdlcResult<Specification> {
        let props = ParserProperties::from([(
            INTEGER_IMPORTS_PROPERTY.to_owned(),
            serde_json::to_value(graph.integer_imports(unit))?,
        )]);
        Ok(Pipeline::lower_spec(
            unit.typed_ast().clone(),
            &props,
            unit.file(),
            resolver,
        )?)
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
