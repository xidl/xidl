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
        let spec = Specification::from_typed_ast_with_resolver(
            typed,
            Default::default(),
            path,
            &mut *resolver,
        )?;
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

    /// Returns the analyzed versioning record for this contract.
    pub(crate) fn record(&self) -> &VersioningRecord {
        &self.record
    }

    /// Formats a canonical projection with the `@version` enum excluded so
    /// adding or removing later timeline members never mutates frozen versions.
    pub(crate) fn formatted_lock_projection(&self, version: VersionId) -> IdlcResult<String> {
        let mut definitions = self.projected_definitions(version)?;
        if let Some(path) = Self::find_version_enum_path(&self.spec.0, &[]) {
            Self::strip_enum_at_path(&mut definitions, &path);
        }
        let body = format_idl_source(&IdlPrinter::render(&Specification(definitions)))?;
        Ok(self.imports.clone() + &body)
    }

    fn formatted_projection(&self, version: VersionId) -> IdlcResult<String> {
        let definitions = self.projected_definitions(version)?;
        let body = format_idl_source(&IdlPrinter::render(&Specification(definitions)))?;
        Ok(self.imports.clone() + &body)
    }

    fn projected_definitions(&self, version: VersionId) -> IdlcResult<Vec<Definition>> {
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
        Ok(definitions)
    }

    fn find_version_enum_path(definitions: &[Definition], scope: &[String]) -> Option<Vec<String>> {
        for definition in definitions {
            match definition {
                Definition::ModuleDcl(module) => {
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    if let Some(found) = Self::find_version_enum_path(&module.definition, &nested) {
                        return Some(found);
                    }
                }
                Definition::TypeDcl(xidl_parser::hir::TypeDcl::ConstrTypeDcl(
                    xidl_parser::hir::ConstrTypeDcl::EnumDcl(enumeration),
                ))
                | Definition::ConstrTypeDcl(xidl_parser::hir::ConstrTypeDcl::EnumDcl(
                    enumeration,
                )) if enumeration.annotations.iter().any(|annotation| {
                    matches!(
                        annotation,
                        xidl_parser::hir::Annotation::Version
                            | xidl_parser::hir::Annotation::VersionParameter(_)
                    )
                }) =>
                {
                    let mut path = scope.to_vec();
                    path.push(enumeration.ident.clone());
                    return Some(path);
                }
                _ => {}
            }
        }
        None
    }

    fn strip_enum_at_path(definitions: &mut Vec<Definition>, path: &[String]) {
        let Some((head, tail)) = path.split_first() else {
            return;
        };
        if tail.is_empty() {
            definitions.retain(|definition| {
                !matches!(
                    definition,
                    Definition::TypeDcl(xidl_parser::hir::TypeDcl::ConstrTypeDcl(
                        xidl_parser::hir::ConstrTypeDcl::EnumDcl(enumeration),
                    )) | Definition::ConstrTypeDcl(xidl_parser::hir::ConstrTypeDcl::EnumDcl(
                        enumeration,
                    )) if enumeration.ident == *head
                )
            });
            return;
        }
        for definition in definitions {
            if let Definition::ModuleDcl(module) = definition
                && module.ident == *head
            {
                Self::strip_enum_at_path(&mut module.definition, tail);
            }
        }
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
