use crate::error::{IdlcError, IdlcResult};
use crate::fmt::format_idl_source;
use crate::library::LibraryLoader;
use crate::units::UnitGraph;
use similar::TextDiff;
use std::path::Path;
use xidl_parser::hir::printer::IdlPrinter;
use xidl_parser::hir::{Definition, Pragma, Specification};
use xidl_parser::parser::parser_text_with_resolver;
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
        let inputs = [path.to_path_buf()];
        let resolved = LibraryLoader::new().resolve(&inputs)?;
        resolved.check_compiler()?;
        let graph = UnitGraph::build(&inputs, resolved.mounts())?;
        let unit = graph.units().last().ok_or_else(|| {
            IdlcError::fmt(format!(
                "version diff: contract '{}' has no unit",
                path.display()
            ))
        })?;
        let mut resolver = crate::library::include_resolver(resolved.mounts());
        let typed = parser_text_with_resolver(unit.source(), path.to_str(), &mut *resolver)?;
        let properties = [("expand_interface".into(), false.into())].into();
        let spec =
            Specification::from_typed_ast_with_resolver(typed, properties, path, &mut *resolver)?;
        let record = VersioningRecord::analyze(&spec)?.ok_or_else(|| {
            IdlcError::fmt(format!(
                "version diff: contract '{}' requires a @version enum",
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
        Ok(Self {
            spec,
            record,
            imports,
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

    fn formatted_projection(&self, version: VersionId) -> IdlcResult<String> {
        let projected = self.record.project(&self.spec, version)?;
        // Projection's outer module and document version identify an artifact,
        // while the diff header already identifies each side. Keep original
        // modules inside that wrapper and every other contract pragma.
        let mut definitions = Vec::new();
        for definition in projected.0 {
            match definition {
                Definition::Pragma(Pragma::XidlcOpenApiVersion(_)) => {}
                Definition::ModuleDcl(module) => definitions.extend(module.definition),
                definition => definitions.push(definition),
            }
        }
        let body = format_idl_source(&IdlPrinter::render(&Specification(definitions)))?;
        Ok(self.imports.clone() + &body)
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
