use super::{
    Definition, FsIncludeResolver, InterfaceDcl, ModuleDcl, ParserProperties, Specification,
    TypeDcl, VersionedIncludes, expand_annotations, parse_xidlc_pragma,
};
use crate::jsonrpc_hir;
use crate::parser::IncludeResolver;
use crate::rest_hir::{self, HirProjectionKind, ProjectedHir};
use crate::semantic;
use crate::versioning::{self, VersionedProjection, VersioningRecord};
use serde_json::Value;
use std::path::Path;

impl From<crate::typed_ast::Specification> for Specification {
    fn from(value: crate::typed_ast::Specification) -> Self {
        let mut resolver = FsIncludeResolver;
        let mut includes = VersionedIncludes::new(&mut resolver, None);
        spec_from_typed_ast(value, &mut includes)
    }
}

impl Specification {
    /// Lowers source declarations for an export index without evaluating them.
    ///
    /// Includes use the caller's resolver. Version projection and semantic
    /// analysis remain the responsibility of compilation.
    pub fn source_from_typed_ast_with_resolver(
        value: crate::typed_ast::Specification,
        path: impl AsRef<Path>,
        resolver: &mut dyn IncludeResolver,
    ) -> crate::error::ParserResult<Self> {
        let current = Self::current_path(path.as_ref());
        let mut includes = VersionedIncludes::new(resolver, current);
        Self::lower_source(value, &mut includes)
    }

    pub fn from_typed_ast_with_properties_and_path(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        let mut resolver = FsIncludeResolver;
        Self::from_typed_ast_with_resolver(value, properties, path, &mut resolver)
    }

    /// Lowers an unprojected typed AST using the caller's include resolver.
    ///
    /// Preserves dependency mounts when a versioned include is resolved during
    /// lowering, after the outer source has already been parsed.
    pub fn from_typed_ast_with_resolver(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        path: impl AsRef<Path>,
        resolver: &mut dyn IncludeResolver,
    ) -> crate::error::ParserResult<Self> {
        let current = Self::current_path(path.as_ref());
        let mut includes = VersionedIncludes::new(resolver, current);
        let imports = semantic::IntegerImports::from_properties(&properties)?;
        Self::lower(value, &mut includes, &imports)
    }

    pub fn from_typed_ast_with_path(
        value: crate::typed_ast::Specification,
        path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        let mut resolver = FsIncludeResolver;
        let current = Self::current_path(path.as_ref());
        let mut includes = VersionedIncludes::new(&mut resolver, current);
        Self::lower(value, &mut includes, &semantic::IntegerImports::default())
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
        path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Vec<VersionedProjection>> {
        let mut resolver = FsIncludeResolver;
        Self::project_typed_ast_versions_with_resolver(value, properties, path, &mut resolver)
    }

    /// Lowers a typed AST with `resolver` for the includes it names.
    ///
    /// Versioned includes need the resolver that resolved the unit's own
    /// includes, so a caller that has one passes it here instead of letting the
    /// filesystem resolver stand in.
    ///
    /// # Errors
    ///
    /// Returns an error when a versioned include cannot be resolved, read,
    /// projected, or spliced.
    pub fn project_typed_ast_versions_with_resolver(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        path: impl AsRef<Path>,
        resolver: &mut dyn IncludeResolver,
    ) -> crate::error::ParserResult<Vec<VersionedProjection>> {
        let current = Self::current_path(path.as_ref());
        let mut includes = VersionedIncludes::new(resolver, current);
        let layout = versioning::VersionLayout::from_properties(&properties)?;
        let selection = versioning::selection_from_properties(&properties)?;
        let integer_imports = semantic::IntegerImports::from_properties(&properties)?;
        let source = Self::lower_source(value, &mut includes)?;
        let Some(record) = VersioningRecord::analyze(&source)? else {
            if let Some(selection) = &selection {
                return Err(versioning::unversioned_selection_error(selection));
            }
            let spec = Self::finalize_projection(source, &integer_imports)?;
            return Ok(vec![VersionedProjection {
                versions: Vec::new(),
                version: None,
                hir: Self::project_hir(spec, &properties, None, Vec::new())?,
            }]);
        };
        let versions = versioning::selected_versions(selection.as_ref(), &record)?;
        let version_parameter = record.parameter();
        let version_values = |defs: &[versioning::VersionDef]| {
            defs.iter()
                .filter(|_| version_parameter.is_some())
                .map(|def| (def.ident.clone(), def.value.clone()))
                .collect::<Vec<_>>()
        };
        match layout {
            versioning::VersionLayout::Merged => {
                let selected = versions
                    .iter()
                    .filter_map(|version| record.timeline().get(*version).cloned())
                    .collect::<Vec<_>>();
                let values = version_values(&selected);
                let spec = record.merge_source(&source, &versions)?;
                let spec = Self::finalize_projection(spec, &integer_imports)?;
                Ok(vec![VersionedProjection {
                    versions: selected,
                    version: None,
                    hir: Self::project_hir(spec, &properties, version_parameter, values)?,
                }])
            }
            versioning::VersionLayout::PerVersion => {
                let mut projections = Vec::new();
                for version in versions {
                    let spec = record.project_source(&source, version)?;
                    let spec = Self::finalize_projection(spec, &integer_imports)?;
                    let definition = record.timeline().get(version).cloned();
                    let values = definition
                        .iter()
                        .filter(|_| version_parameter.is_some())
                        .map(|def| (def.ident.clone(), def.value.clone()))
                        .collect();
                    projections.push(VersionedProjection {
                        versions: definition.clone().into_iter().collect(),
                        version: definition,
                        hir: Self::project_hir(spec, &properties, version_parameter, values)?,
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
        version_parameter: Option<&super::VersionParameter>,
        version_values: Vec<(String, String)>,
    ) -> crate::error::ParserResult<ProjectedHir> {
        match hir_projection_kind(properties) {
            HirProjectionKind::Rpc => Ok(ProjectedHir::Rpc(spec)),
            HirProjectionKind::Http => {
                let options = rest_hir::ProjectOptions {
                    version_parameter: version_parameter.cloned(),
                    version_values,
                };
                rest_hir::project_with_options(&spec, &options).map(ProjectedHir::Http)
            }
            HirProjectionKind::JsonRpc => jsonrpc_hir::project(&spec).map(ProjectedHir::JsonRpc),
        }
    }

    /// Builds the source HIR: definitions plus shared source validation.
    fn lower_source(
        value: crate::typed_ast::Specification,
        includes: &mut VersionedIncludes<'_>,
    ) -> crate::error::ParserResult<Self> {
        crate::lint::AnnotationKind::Webhooks.validate_source(&value.0)?;
        let mut definitions = Vec::new();
        collect_defs(value.0, &mut definitions, includes)?;
        let spec = Self(definitions);
        semantic::UnionCaseLabels::validate(&spec.0)?;
        // All source targets share HTTP declaration validation; builtins belong
        // to its temporary type environment, not the generated user models.
        rest_hir::HttpUnion::validate_source(&spec.0)?;
        Ok(spec)
    }

    /// Resolves integer expressions in the selected source declarations.
    fn finalize_projection(
        mut spec: Self,
        imports: &semantic::IntegerImports,
    ) -> crate::error::ParserResult<Self> {
        semantic::IntegerDeclarations::normalize(&mut spec, imports)?;
        Self::finalize(spec, imports)
    }

    /// Validates collection bounds and structure on an already lowered spec.
    fn finalize(
        mut spec: Self,
        imports: &semantic::IntegerImports,
    ) -> crate::error::ParserResult<Self> {
        semantic::CollectionBounds::normalize(&mut spec, imports)?;
        semantic::Analysis::structure(&mut spec)?;
        Ok(spec)
    }

    fn lower(
        value: crate::typed_ast::Specification,
        includes: &mut VersionedIncludes<'_>,
        imports: &semantic::IntegerImports,
    ) -> crate::error::ParserResult<Self> {
        let spec = Self::lower_source(value, includes)?;
        // API versioning annotations are validated on every compile, even when
        // no version is projected.
        VersioningRecord::analyze(&spec)?;
        Self::finalize(spec, imports)
    }

    /// Path handed to include resolution, when it is valid utf-8.
    fn current_path(path: &Path) -> Option<String> {
        path.to_str().map(str::to_string)
    }
}

pub(crate) fn spec_from_typed_ast(
    value: crate::typed_ast::Specification,
    includes: &mut VersionedIncludes<'_>,
) -> Specification {
    Specification::lower(value, includes, &semantic::IntegerImports::default())
        .expect("HIR conversion should not fail")
}

pub(super) fn collect_defs(
    defs: Vec<crate::typed_ast::Definition>,
    out: &mut Vec<Definition>,
    includes: &mut VersionedIncludes<'_>,
) -> crate::error::ParserResult<()> {
    for def in defs {
        match def {
            crate::typed_ast::Definition::ModuleDcl(module) => {
                let ident = module.ident.0;
                let annotations = expand_annotations(module.annotations);
                let mut inner = Vec::new();
                collect_defs(module.definition, &mut inner, includes)?;
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
            crate::typed_ast::Definition::PreprocInclude(include) => {
                // Plain includes are handled at the tree-sitter stage. A
                // versioned include reaches this point and is projected here.
                if let Some(path) = include_path_text(&include) {
                    includes.expand(&path, out)?;
                }
            }
            crate::typed_ast::Definition::ImportDcl(_) => {
                // Imports name another compilation unit. The unit graph owns
                // them, so the lowered HIR of one unit stays unchanged.
            }
            crate::typed_ast::Definition::TemplateModuleDcl(_)
            | crate::typed_ast::Definition::TemplateModuleInst(_)
            | crate::typed_ast::Definition::PreprocDefine(_) => {}
        }
    }
    Ok(())
}

/// The unquoted path of an include declaration, when it was written as one.
fn include_path_text(include: &crate::typed_ast::PreprocInclude) -> Option<String> {
    match &include.path {
        crate::typed_ast::PreprocIncludePath::StringLiteral(text) => {
            Some(text.trim().trim_matches('"').to_string())
        }
        crate::typed_ast::PreprocIncludePath::SystemLibString(_)
        | crate::typed_ast::PreprocIncludePath::Identifier(_) => None,
    }
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
