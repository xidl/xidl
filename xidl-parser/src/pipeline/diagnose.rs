//! Multi-error diagnostic orchestration across syntax, AST, HIR, and protocol passes.

use crate::diagnostic::{
    AstDiagnosticPass, Diagnostic, DiagnosticBag, DiagnosticCode, HirDiagnosticPass,
};
use crate::error::ParseError;
use crate::hir::{Specification, VersionParameter};
use crate::lint::AnnotationLintPass;
use crate::parser::{IncludeResolver, parser_text_with_resolver};
use crate::rest_hir::{self, ProjectOptions};
use crate::semantic::IntegerImports;
use crate::typed_ast;
use crate::versioning::{VersionDef, VersioningRecord};
use std::path::PathBuf;

use super::passes::{
    CollectionBoundsPass, HttpUnionPass, JsonRpcProtocolPass, RestProtocolPass,
    StructInheritancePass, UnionCasePass,
};
use super::versioned_include::VersionedIncludes;
use super::{HirProjectionKind, Pipeline};

/// Configuration for [`Pipeline::diagnose_source`] and [`Pipeline::diagnose_typed_ast`].
#[derive(Debug, Clone)]
pub struct DiagnoseOptions {
    /// Logical source file path used for include resolution.
    pub path: PathBuf,
    /// Optional explicit protocol target; `None` auto-detects HTTP annotations.
    pub protocol: Option<HirProjectionKind>,
    /// Imported integer constants available to collection-bounds checks.
    pub integer_imports: IntegerImports,
}

impl DiagnoseOptions {
    /// Creates diagnostic options for `path` with automatic protocol detection.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            protocol: None,
            integer_imports: IntegerImports::default(),
        }
    }

    /// Selects an explicit protocol projection to validate.
    pub fn with_protocol(mut self, protocol: HirProjectionKind) -> Self {
        self.protocol = Some(protocol);
        self
    }

    /// Supplies imported integer constants for collection-bounds validation.
    pub fn with_integer_imports(mut self, imports: IntegerImports) -> Self {
        self.integer_imports = imports;
        self
    }
}

struct ProjectedCheckContext<'a> {
    options: &'a DiagnoseOptions,
    version_parameter: Option<&'a VersionParameter>,
    version: Option<&'a VersionDef>,
    run_http: bool,
    run_jsonrpc: bool,
}

impl Pipeline {
    /// Runs syntax, AST lint, semantic, versioning, and protocol diagnostic passes on `source`.
    pub fn diagnose_source(
        source: &str,
        options: DiagnoseOptions,
        resolver: &mut dyn IncludeResolver,
    ) -> DiagnosticBag {
        let current = Self::current_path(&options.path);
        match parser_text_with_resolver(source, current.as_deref(), resolver) {
            Ok(ast) => Self::diagnose_typed_ast(ast, options, resolver),
            Err(ParseError::Diagnostics(diagnostics)) => {
                let mut bag = DiagnosticBag::new();
                bag.extend(diagnostics);
                bag
            }
            Err(err) => {
                let mut bag = DiagnosticBag::new();
                bag.push(Diagnostic::error(
                    DiagnosticCode::SyntaxError,
                    0..0,
                    err.to_string(),
                ));
                bag
            }
        }
    }

    /// Runs AST lint, semantic, versioning, and protocol diagnostic passes on `ast`.
    pub fn diagnose_typed_ast(
        ast: typed_ast::Specification,
        options: DiagnoseOptions,
        resolver: &mut dyn IncludeResolver,
    ) -> DiagnosticBag {
        let mut bag = DiagnosticBag::new();
        AnnotationLintPass::all().run(&ast, &mut bag);

        let current = Self::current_path(&options.path);
        let mut includes = VersionedIncludes::new(resolver, current);
        let source_spec = match Specification::lower_raw(ast, &mut includes) {
            Ok(spec) => spec,
            Err(err) => {
                bag.push(Diagnostic::error(
                    DiagnosticCode::SemanticError,
                    0..0,
                    err.to_string(),
                ));
                bag.dedup();
                return bag;
            }
        };

        UnionCasePass.run(&source_spec, &mut bag);
        HttpUnionPass.run(&source_spec, &mut bag);
        let record = VersioningRecord::collect_diagnostics(&source_spec, &mut bag);

        let run_http = match options.protocol {
            Some(HirProjectionKind::Http) => true,
            Some(HirProjectionKind::JsonRpc | HirProjectionKind::Rpc) => false,
            None => rest_hir::diagnostics::has_http_annotations(&source_spec),
        };
        let run_jsonrpc = match options.protocol {
            Some(HirProjectionKind::JsonRpc) => true,
            Some(HirProjectionKind::Http | HirProjectionKind::Rpc) => false,
            None => !run_http,
        };

        if let Some(record) = record {
            let version_parameter = record.parameter();
            for (version_id, version_def) in record.timeline().iter() {
                match record.project_source(&source_spec, version_id) {
                    Ok(projected) => {
                        let ctx = ProjectedCheckContext {
                            options: &options,
                            version_parameter,
                            version: Some(version_def),
                            run_http,
                            run_jsonrpc,
                        };
                        Self::diagnose_projected(&projected, &ctx, &mut bag);
                    }
                    Err(err) => {
                        bag.push(
                            Diagnostic::error(
                                DiagnosticCode::VersioningError,
                                0..0,
                                err.to_string(),
                            )
                            .with_location(version_def.ident.clone()),
                        );
                    }
                }
            }
        } else {
            let ctx = ProjectedCheckContext {
                options: &options,
                version_parameter: None,
                version: None,
                run_http,
                run_jsonrpc,
            };
            Self::diagnose_projected(&source_spec, &ctx, &mut bag);
        }

        bag.dedup();
        bag
    }

    fn diagnose_projected(
        spec: &Specification,
        ctx: &ProjectedCheckContext<'_>,
        bag: &mut DiagnosticBag,
    ) {
        CollectionBoundsPass::new(ctx.options.integer_imports.clone()).run(spec, bag);
        StructInheritancePass.run(spec, bag);
        if ctx.run_http {
            let version_values = ctx
                .version
                .into_iter()
                .filter(|_| ctx.version_parameter.is_some())
                .map(|def| (def.ident.clone(), def.value.clone()))
                .collect();
            let options = ProjectOptions {
                version_parameter: ctx.version_parameter.cloned(),
                version_values,
            };
            RestProtocolPass::new(options).run(spec, bag);
        }
        if ctx.run_jsonrpc {
            JsonRpcProtocolPass.run(spec, bag);
        }
    }
}
