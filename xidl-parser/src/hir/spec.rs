use super::{
    Definition, FsIncludeResolver, InterfaceDcl, ModuleDcl, ParserProperties, Specification,
    TypeDcl, VersionedIncludeExpander, expand_annotations, parse_xidlc_pragma,
};
use crate::parser::IncludeResolver;
use crate::pipeline::{CompileOptions, Pipeline, ProjectedHir};
use crate::versioning::VersionedProjection;
use std::path::Path;

impl From<crate::typed_ast::Specification> for Specification {
    fn from(value: crate::typed_ast::Specification) -> Self {
        Pipeline::lower_spec_default(value).expect("HIR conversion should not fail")
    }
}

impl Specification {
    /// Lowers a typed AST into raw HIR definitions without running validation passes.
    pub(crate) fn lower_raw(
        value: crate::typed_ast::Specification,
        includes: &mut dyn VersionedIncludeExpander,
    ) -> crate::error::ParserResult<Self> {
        let mut definitions = Vec::new();
        collect_defs(value.0, &mut definitions, includes)?;
        Ok(Self(definitions))
    }

    /// Lowers source declarations for an export index without evaluating them.
    ///
    /// Includes use the caller's resolver. Version projection and semantic
    /// analysis remain the responsibility of compilation.
    pub fn source_from_typed_ast_with_resolver(
        value: crate::typed_ast::Specification,
        path: impl AsRef<Path>,
        resolver: &mut dyn IncludeResolver,
    ) -> crate::error::ParserResult<Self> {
        Pipeline::lower_source_spec(value, path.as_ref(), resolver)
    }

    /// Lowers an unprojected typed AST using the default filesystem include resolver.
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
        Pipeline::lower_spec(value, &properties, path.as_ref(), resolver)
    }

    /// Lowers an unprojected typed AST with default properties and filesystem resolver.
    pub fn from_typed_ast_with_path(
        value: crate::typed_ast::Specification,
        path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        let mut resolver = FsIncludeResolver;
        Pipeline::lower_spec(
            value,
            &ParserProperties::default(),
            path.as_ref(),
            &mut resolver,
        )
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
        let options = CompileOptions::new(path.as_ref(), properties);
        Pipeline::compile_typed_ast(value, options, resolver).map(|unit| unit.projections)
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
}

pub(super) fn collect_defs(
    defs: Vec<crate::typed_ast::Definition>,
    out: &mut Vec<Definition>,
    includes: &mut dyn VersionedIncludeExpander,
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
