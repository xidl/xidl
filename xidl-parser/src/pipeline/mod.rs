//! Unidirectional compilation pipeline from IDL source to projected IR.
//!
//! Coordinates parsing, diagnostic passes, HIR lowering, semantic analysis,
//! version projection, version-map construction, and protocol projection in a
//! single pass without cyclic dependencies inside lower layers.

mod passes;
#[cfg(test)]
mod tests;
mod versioned_include;

use crate::diagnostic::DiagnosticBag;
use crate::error::{ParseError, ParserResult};
use crate::hir::{FsIncludeResolver, ParserProperties, Specification, VersionParameter};
use crate::jsonrpc_hir;
use crate::parser::{IncludeResolver, parser_text_with_resolver};
use crate::rest_hir;
use crate::semantic;
use crate::typed_ast;
use crate::versioning::map::{MapOptions, VersionMap};
use crate::versioning::{self, VersionDef, VersionedProjection, VersioningRecord};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub use passes::{HttpUnionPass, UnionCasePass};
use passes::{SourceAstPasses, SourceHirPasses};
use versioned_include::VersionedIncludes;

/// Selects the projected HIR shape produced from typed AST input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirProjectionKind {
    /// Language-neutral RPC HIR (`hir::Specification`).
    Rpc,
    /// Shared HTTP semantic model (`rest_hir::RestHirDocument`).
    Http,
    /// Shared JSON-RPC semantic model (`jsonrpc_hir::JsonRpcHirDocument`).
    JsonRpc,
}

/// A source specification or its HTTP or JSON-RPC projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectedHir {
    /// Language-neutral RPC specification.
    Rpc(Specification),
    /// Projected HTTP REST document.
    Http(rest_hir::RestHirDocument),
    /// Projected JSON-RPC document.
    JsonRpc(jsonrpc_hir::JsonRpcHirDocument),
}

/// Configuration for a single-pass [`Pipeline::compile_source`] invocation.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Parser and generator properties.
    pub properties: ParserProperties,
    /// Logical source file path used for include resolution and contract naming.
    pub path: PathBuf,
    /// Whether to build a [`VersionMap`] alongside projected outputs.
    pub include_version_map: bool,
    /// Whether HTTP operations should be included in the built [`VersionMap`].
    pub include_http_operations: bool,
}

impl CompileOptions {
    /// Creates compilation options for `path` with `properties`.
    pub fn new(path: impl Into<PathBuf>, properties: ParserProperties) -> Self {
        Self {
            properties,
            path: path.into(),
            include_version_map: false,
            include_http_operations: false,
        }
    }

    /// Enables [`VersionMap`] generation from the same lowered source HIR.
    pub fn with_version_map(mut self, include_http_operations: bool) -> Self {
        self.include_version_map = true;
        self.include_http_operations = include_http_operations;
        self
    }
}

/// Output of a single-pass compilation through [`Pipeline`].
#[derive(Debug)]
pub struct CompiledUnit {
    /// Projected HIR outputs (one per selected version or one merged output).
    pub projections: Vec<VersionedProjection>,
    /// Adjacent-version metadata when the unit declares a `@version` timeline.
    pub version_map: Option<VersionMap>,
    /// Diagnostics accumulated across syntax, AST, and HIR passes.
    pub diagnostics: DiagnosticBag,
}

/// Coordinates syntax, AST, HIR, semantic, versioning, and protocol passes.
pub struct Pipeline;

struct ProjectionRequest<'a> {
    properties: &'a ParserProperties,
    layout: versioning::VersionLayout,
    selection: Option<&'a versioning::VersionSelection>,
    integer_imports: &'a semantic::IntegerImports,
}

impl Pipeline {
    /// Compiles raw IDL `source` in a single parse and lowering pass.
    pub fn compile_source(
        source: &str,
        options: CompileOptions,
        resolver: &mut dyn IncludeResolver,
    ) -> ParserResult<CompiledUnit> {
        let current = Self::current_path(&options.path);
        let typed = parser_text_with_resolver(source, current.as_deref(), resolver)?;
        Self::compile_typed_ast(typed, options, resolver)
    }

    /// Compiles an already-parsed [`typed_ast::Specification`] in one pass.
    pub fn compile_typed_ast(
        typed: typed_ast::Specification,
        options: CompileOptions,
        resolver: &mut dyn IncludeResolver,
    ) -> ParserResult<CompiledUnit> {
        let mut bag = DiagnosticBag::new();
        let source_spec = Self::lower_source_with_bag(typed, &options.path, resolver, &mut bag)?;
        let layout = versioning::VersionLayout::from_properties(&options.properties)?;
        let selection = versioning::selection_from_properties(&options.properties)?;
        let integer_imports = semantic::IntegerImports::from_properties(&options.properties)?;

        let Some(record) = VersioningRecord::analyze(&source_spec)? else {
            if let Some(selection) = &selection {
                return Err(versioning::unversioned_selection_error(selection));
            }
            let spec = Self::finalize_projection(source_spec, &integer_imports)?;
            let hir = Self::project_hir(spec, &options.properties, None, Vec::new())?;
            return Ok(CompiledUnit {
                projections: vec![VersionedProjection {
                    versions: Vec::new(),
                    version: None,
                    hir,
                }],
                version_map: None,
                diagnostics: bag,
            });
        };

        let projections = Self::build_versioned_projections(
            &source_spec,
            &record,
            &ProjectionRequest {
                properties: &options.properties,
                layout,
                selection: selection.as_ref(),
                integer_imports: &integer_imports,
            },
        )?;

        let version_map = if options.include_version_map {
            let contract = options
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| options.path.to_string_lossy().into_owned());
            Some(versioning::map::from_spec_with_http(
                &source_spec,
                &record,
                MapOptions {
                    contract,
                    integer_imports: integer_imports.clone(),
                    include_http_operations: options.include_http_operations,
                },
            )?)
        } else {
            None
        };

        Ok(CompiledUnit {
            projections,
            version_map,
            diagnostics: bag,
        })
    }

    /// Builds only the [`VersionMap`] for `source` in a single parse and lowering pass.
    pub fn build_version_map(
        source: &str,
        options: CompileOptions,
        resolver: &mut dyn IncludeResolver,
    ) -> ParserResult<Option<VersionMap>> {
        let current = Self::current_path(&options.path);
        let typed = parser_text_with_resolver(source, current.as_deref(), resolver)?;
        let mut bag = DiagnosticBag::new();
        let source_spec = Self::lower_source_with_bag(typed, &options.path, resolver, &mut bag)?;
        let Some(record) = VersioningRecord::analyze(&source_spec)? else {
            return Ok(None);
        };
        let contract = options
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .unwrap_or_else(|| options.path.to_string_lossy().into_owned());
        let integer_imports = semantic::IntegerImports::from_properties(&options.properties)?;
        versioning::map::from_spec_with_http(
            &source_spec,
            &record,
            MapOptions {
                contract,
                integer_imports,
                include_http_operations: options.include_http_operations,
            },
        )
        .map(Some)
    }

    /// Lowers source declarations without evaluating integer constants or version projection.
    pub fn lower_source_spec(
        value: typed_ast::Specification,
        path: &Path,
        resolver: &mut dyn IncludeResolver,
    ) -> ParserResult<Specification> {
        let mut bag = DiagnosticBag::new();
        Self::lower_source_with_bag(value, path, resolver, &mut bag)
    }

    /// Lowers a typed AST through source validation, versioning validation, and semantic analysis.
    pub fn lower_spec(
        value: typed_ast::Specification,
        properties: &ParserProperties,
        path: &Path,
        resolver: &mut dyn IncludeResolver,
    ) -> ParserResult<Specification> {
        let mut bag = DiagnosticBag::new();
        let spec = Self::lower_source_with_bag(value, path, resolver, &mut bag)?;
        VersioningRecord::analyze(&spec)?;
        let imports = semantic::IntegerImports::from_properties(properties)?;
        Self::finalize(spec, &imports)
    }

    /// Lowers a typed AST using the default filesystem resolver.
    pub fn lower_spec_default(value: typed_ast::Specification) -> ParserResult<Specification> {
        let mut resolver = FsIncludeResolver;
        let mut includes = VersionedIncludes::new(&mut resolver, None);
        let mut bag = DiagnosticBag::new();
        SourceAstPasses::run(&value, &mut bag);
        Self::check_ast_bag(&bag)?;
        let spec = Specification::lower_raw(value, &mut includes)?;
        SourceHirPasses::run(&spec, &mut bag);
        bag.check_errors()?;
        VersioningRecord::analyze(&spec)?;
        Self::finalize(spec, &semantic::IntegerImports::default())
    }

    fn lower_source_with_bag(
        value: typed_ast::Specification,
        path: &Path,
        resolver: &mut dyn IncludeResolver,
        bag: &mut DiagnosticBag,
    ) -> ParserResult<Specification> {
        SourceAstPasses::run(&value, bag);
        Self::check_ast_bag(bag)?;
        let current = Self::current_path(path);
        let mut includes = VersionedIncludes::new(resolver, current);
        let spec = Specification::lower_raw(value, &mut includes)?;
        SourceHirPasses::run(&spec, bag);
        bag.check_errors()?;
        Ok(spec)
    }

    fn check_ast_bag(bag: &DiagnosticBag) -> ParserResult<()> {
        if let Some(diagnostic) = bag
            .as_slice()
            .iter()
            .find(|item| item.severity == crate::diagnostic::Severity::Error)
        {
            return Err(ParseError::Message(format!(
                "xidl annotation at bytes {}..{}: {}",
                diagnostic.span.start, diagnostic.span.end, diagnostic.message
            )));
        }
        Ok(())
    }

    fn build_versioned_projections(
        source: &Specification,
        record: &VersioningRecord,
        request: &ProjectionRequest<'_>,
    ) -> ParserResult<Vec<VersionedProjection>> {
        let versions = versioning::selected_versions(request.selection, record)?;
        let version_parameter = record.parameter();
        let version_values = |defs: &[VersionDef]| {
            defs.iter()
                .filter(|_| version_parameter.is_some())
                .map(|def| (def.ident.clone(), def.value.clone()))
                .collect::<Vec<_>>()
        };
        match request.layout {
            versioning::VersionLayout::Merged => {
                let selected = versions
                    .iter()
                    .filter_map(|version| record.timeline().get(*version).cloned())
                    .collect::<Vec<_>>();
                let values = version_values(&selected);
                let spec = record.merge_source(source, &versions)?;
                let spec = Self::finalize_projection(spec, request.integer_imports)?;
                Ok(vec![VersionedProjection {
                    versions: selected,
                    version: None,
                    hir: Self::project_hir(spec, request.properties, version_parameter, values)?,
                }])
            }
            versioning::VersionLayout::PerVersion => {
                let mut projections = Vec::new();
                for version in versions {
                    let spec = record.project_source(source, version)?;
                    let spec = Self::finalize_projection(spec, request.integer_imports)?;
                    let definition = record.timeline().get(version).cloned();
                    let values = definition
                        .iter()
                        .filter(|_| version_parameter.is_some())
                        .map(|def| (def.ident.clone(), def.value.clone()))
                        .collect();
                    projections.push(VersionedProjection {
                        versions: definition.clone().into_iter().collect(),
                        version: definition,
                        hir: Self::project_hir(
                            spec,
                            request.properties,
                            version_parameter,
                            values,
                        )?,
                    });
                }
                Ok(projections)
            }
        }
    }

    fn project_hir(
        spec: Specification,
        properties: &ParserProperties,
        version_parameter: Option<&VersionParameter>,
        version_values: Vec<(String, String)>,
    ) -> ParserResult<ProjectedHir> {
        match Self::hir_projection_kind(properties) {
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

    fn finalize_projection(
        mut spec: Specification,
        imports: &semantic::IntegerImports,
    ) -> ParserResult<Specification> {
        semantic::IntegerDeclarations::normalize(&mut spec, imports)?;
        Self::finalize(spec, imports)
    }

    fn finalize(
        mut spec: Specification,
        imports: &semantic::IntegerImports,
    ) -> ParserResult<Specification> {
        semantic::CollectionBounds::normalize(&mut spec, imports)?;
        semantic::Analysis::structure(&mut spec)?;
        Ok(spec)
    }

    fn current_path(path: &Path) -> Option<String> {
        path.to_str().map(str::to_string)
    }

    fn hir_projection_kind(properties: &ParserProperties) -> HirProjectionKind {
        match properties.get("hir_kind").and_then(Value::as_str) {
            Some(value) if value.eq_ignore_ascii_case("http") => HirProjectionKind::Http,
            Some(value)
                if value.eq_ignore_ascii_case("jsonrpc")
                    || value.eq_ignore_ascii_case("json-rpc") =>
            {
                HirProjectionKind::JsonRpc
            }
            _ => HirProjectionKind::Rpc,
        }
    }
}
