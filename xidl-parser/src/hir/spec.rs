use super::{
    Definition, InterfaceDcl, ModuleDcl, ParserProperties, Specification, TypeDcl,
    expand_annotations, interface_codegen, parse_xidlc_pragma,
};
use crate::jsonrpc_hir;
use crate::rest_hir::{self, HirProjectionKind, ProjectedHir};
use crate::semantic;
use crate::versioning::{self, VersionedProjection, VersioningRecord};
use serde_json::Value;
use std::path::Path;

impl From<crate::typed_ast::Specification> for Specification {
    fn from(value: crate::typed_ast::Specification) -> Self {
        spec_from_typed_ast(value, true)
    }
}

impl Specification {
    pub fn from_typed_ast_with_properties(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
    ) -> Self {
        spec_from_typed_ast(value, expand_interface(&properties))
    }

    pub fn from_typed_ast_with_properties_and_path(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        _path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        Self::lower(value, expand_interface(&properties))
    }

    pub fn from_typed_ast_with_path(
        value: crate::typed_ast::Specification,
        _path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        Self::lower(value, true)
    }

    /// Lowers a typed AST and returns one HIR projection per requested version.
    ///
    /// Unversioned specifications produce exactly one projection without a
    /// version. Versioned specifications produce one merged projection holding
    /// one module per selected version (`merged` layout, the default), or one
    /// projection per selected version (`per_version` layout).
    pub fn project_typed_ast_versions(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        _path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Vec<VersionedProjection>> {
        let layout = versioning::VersionLayout::from_properties(&properties)?;
        let selection = versioning::selection_from_properties(&properties)?;
        let expand = expand_interface(&properties);
        let source = Self::lower_source(value)?;
        let Some(record) = VersioningRecord::analyze(&source)? else {
            if let Some(selection) = &selection {
                return Err(versioning::unversioned_selection_error(selection));
            }
            let spec = Self::finalize(source, expand)?;
            return Ok(vec![VersionedProjection {
                versions: Vec::new(),
                version: None,
                hir: Self::project_hir(spec, &properties)?,
            }]);
        };
        let versions = versioning::selected_versions(selection.as_ref(), &record)?;
        match layout {
            versioning::VersionLayout::Merged => {
                let spec = record.merge(&source, &versions)?;
                let spec = Self::finalize(spec, expand)?;
                let selected = versions
                    .iter()
                    .filter_map(|version| record.timeline().get(*version).cloned())
                    .collect();
                Ok(vec![VersionedProjection {
                    versions: selected,
                    version: None,
                    hir: Self::project_hir(spec, &properties)?,
                }])
            }
            versioning::VersionLayout::PerVersion => {
                let mut projections = Vec::new();
                for version in versions {
                    let spec = record.project(&source, version)?;
                    let spec = Self::finalize(spec, expand)?;
                    let definition = record.timeline().get(version).cloned();
                    projections.push(VersionedProjection {
                        versions: definition.clone().into_iter().collect(),
                        version: definition,
                        hir: Self::project_hir(spec, &properties)?,
                    });
                }
                Ok(projections)
            }
        }
    }

    /// Lowers a typed AST and projects the first request of the merged layout.
    ///
    /// Callers that need per-version output use [`Self::project_typed_ast_versions`].
    pub fn project_typed_ast_with_properties_and_path(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<ProjectedHir> {
        let mut projections = Self::project_typed_ast_versions(value, properties, path)?;
        match projections.len() {
            1 => Ok(projections.remove(0).hir),
            _ => Err(crate::error::ParseError::Message(
                "xidl versioning: per-version layout requires project_typed_ast_versions"
                    .to_string(),
            )),
        }
    }

    fn project_hir(
        spec: Specification,
        properties: &ParserProperties,
    ) -> crate::error::ParserResult<ProjectedHir> {
        match hir_projection_kind(properties) {
            HirProjectionKind::Rpc => Ok(ProjectedHir::Rpc(spec)),
            HirProjectionKind::Http => rest_hir::project(&spec).map(ProjectedHir::Http),
            HirProjectionKind::JsonRpc => jsonrpc_hir::project(&spec).map(ProjectedHir::JsonRpc),
        }
    }

    /// Builds the source HIR: definitions plus shared source validation.
    fn lower_source(value: crate::typed_ast::Specification) -> crate::error::ParserResult<Self> {
        let mut definitions = Vec::new();
        collect_defs(value.0, &mut definitions);
        let spec = Self(definitions);
        // All source targets share HTTP declaration validation; builtins belong
        // to its temporary type environment, not the generated user models.
        rest_hir::HttpUnion::validate_source(&spec.0)?;
        Ok(spec)
    }

    /// Runs interface expansion and semantic analysis on an already lowered spec.
    fn finalize(mut spec: Self, expand_interfaces: bool) -> crate::error::ParserResult<Self> {
        // Validate lexical source scopes before adding RPC wrapper declarations
        // at their generated scopes.
        if expand_interfaces {
            Self::expand_interfaces(&mut spec.0, &mut Vec::new())?;
        }
        semantic::analyze(&mut spec);
        Ok(spec)
    }

    fn lower(
        value: crate::typed_ast::Specification,
        expand_interfaces: bool,
    ) -> crate::error::ParserResult<Self> {
        let spec = Self::lower_source(value)?;
        // API versioning annotations are validated on every compile, even when
        // no version is projected.
        VersioningRecord::analyze(&spec)?;
        Self::finalize(spec, expand_interfaces)
    }

    fn expand_interfaces(
        definitions: &mut Vec<Definition>,
        modules: &mut Vec<String>,
    ) -> crate::error::ParserResult<()> {
        for mut definition in std::mem::take(definitions) {
            match &mut definition {
                Definition::ModuleDcl(module) => {
                    modules.push(module.ident.clone());
                    Self::expand_interfaces(&mut module.definition, modules)?;
                    modules.pop();
                }
                Definition::InterfaceDcl(interface) => {
                    definitions.extend(interface_codegen::expand_interface(interface, modules)?);
                }
                _ => {}
            }
            definitions.push(definition);
        }
        Ok(())
    }
}

pub(crate) fn spec_from_typed_ast(
    value: crate::typed_ast::Specification,
    expand_interfaces: bool,
) -> Specification {
    Specification::lower(value, expand_interfaces).expect("HIR conversion should not fail")
}

pub(super) fn collect_defs(defs: Vec<crate::typed_ast::Definition>, out: &mut Vec<Definition>) {
    for def in defs {
        match def {
            crate::typed_ast::Definition::ModuleDcl(module) => {
                let ident = module.ident.0;
                let annotations = expand_annotations(module.annotations);
                let mut inner = Vec::new();
                collect_defs(module.definition, &mut inner);
                out.push(Definition::ModuleDcl(ModuleDcl {
                    annotations,
                    ident,
                    definition: inner,
                }));
            }
            crate::typed_ast::Definition::PreprocCall(call) => {
                if let Some(pragma) = parse_xidlc_pragma(&call) {
                    out.push(Definition::Pragma(pragma));
                }
            }
            crate::typed_ast::Definition::TypeDcl(value) => {
                out.push(Definition::TypeDcl(TypeDcl::from(value)))
            }
            crate::typed_ast::Definition::ConstDcl(value) => {
                out.push(Definition::ConstDcl(value.into()))
            }
            crate::typed_ast::Definition::ExceptDcl(value) => {
                out.push(Definition::ExceptDcl(value.into()))
            }
            crate::typed_ast::Definition::InterfaceDcl(value) => {
                out.push(Definition::InterfaceDcl(InterfaceDcl::from(value)));
            }
            crate::typed_ast::Definition::PreprocInclude(_) => {
                // Includes are now handled at the tree-sitter stage.
                // If we see a PreprocInclude here, it means it was not expanded.
            }
            crate::typed_ast::Definition::TemplateModuleDcl(_)
            | crate::typed_ast::Definition::TemplateModuleInst(_)
            | crate::typed_ast::Definition::PreprocDefine(_) => {}
        }
    }
}

fn expand_interface(properties: &ParserProperties) -> bool {
    if let Some(expand) = properties.get("expand_interface").and_then(Value::as_bool) {
        return expand;
    }

    matches!(hir_projection_kind(properties), HirProjectionKind::Rpc)
}

fn hir_projection_kind(properties: &ParserProperties) -> HirProjectionKind {
    match properties.get("hir_kind").and_then(Value::as_str) {
        Some(value) if value.eq_ignore_ascii_case("http") => HirProjectionKind::Http,
        Some(value)
            if value.eq_ignore_ascii_case("jsonrpc") || value.eq_ignore_ascii_case("json-rpc") =>
        {
            HirProjectionKind::JsonRpc
        }
        _ => HirProjectionKind::Rpc,
    }
}
