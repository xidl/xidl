//! Diagnostic passes executed by the parser compilation pipeline.

use crate::annotations::AnnotationKind;
use crate::diagnostic::{AstDiagnosticPass, DiagnosticBag, HirDiagnosticPass};
use crate::hir::Specification;
use crate::jsonrpc_hir;
use crate::lint::AnnotationLintPass;
use crate::rest_hir::{self, HttpUnion, ProjectOptions};
use crate::semantic::{CollectionBounds, IntegerImports, StructInheritance, UnionCaseLabels};
use crate::typed_ast;
use crate::versioning::VersioningRecord;

/// Runs source-required AST diagnostic passes before HIR lowering.
pub(crate) struct SourceAstPasses;

impl SourceAstPasses {
    /// Runs built-in AST validation passes into `bag`.
    pub(crate) fn run(ast: &typed_ast::Specification, bag: &mut DiagnosticBag) {
        AnnotationLintPass::for_kind(AnnotationKind::Webhooks).run(ast, bag);
    }
}

/// Validates union case labels on a lowered specification.
pub struct UnionCasePass;

impl HirDiagnosticPass for UnionCasePass {
    fn run(&self, spec: &Specification, bag: &mut DiagnosticBag) {
        UnionCaseLabels::collect_diagnostics(&spec.0, bag);
    }
}

/// Validates `@http` union declarations on a lowered specification.
pub struct HttpUnionPass;

impl HirDiagnosticPass for HttpUnionPass {
    fn run(&self, spec: &Specification, bag: &mut DiagnosticBag) {
        HttpUnion::collect_diagnostics(&spec.0, true, bag);
    }
}

/// Validates single-base struct inheritance rules on a lowered specification.
pub struct StructInheritancePass;

impl HirDiagnosticPass for StructInheritancePass {
    fn run(&self, spec: &Specification, bag: &mut DiagnosticBag) {
        StructInheritance::collect_diagnostics(spec, bag);
    }
}

/// Validates sequence, map, and array bounds on a lowered specification.
#[derive(Debug, Default, Clone)]
pub struct CollectionBoundsPass {
    imports: IntegerImports,
}

impl CollectionBoundsPass {
    /// Creates a collection-bounds pass using imported integer constants.
    pub fn new(imports: IntegerImports) -> Self {
        Self { imports }
    }
}

impl HirDiagnosticPass for CollectionBoundsPass {
    fn run(&self, spec: &Specification, bag: &mut DiagnosticBag) {
        CollectionBounds::collect_diagnostics(spec, &self.imports, bag);
    }
}

/// Validates `@version` timelines, lifecycle intervals, and wire-name claims.
pub struct VersioningPass;

impl HirDiagnosticPass for VersioningPass {
    fn run(&self, spec: &Specification, bag: &mut DiagnosticBag) {
        let _ = VersioningRecord::collect_diagnostics(spec, bag);
    }
}

/// Validates HTTP/REST projection rules across all exceptions, unions, and interfaces.
#[derive(Debug, Default)]
pub struct RestProtocolPass {
    options: ProjectOptions,
}

impl RestProtocolPass {
    /// Creates an HTTP/REST protocol pass with explicit projection options.
    pub fn new(options: ProjectOptions) -> Self {
        Self { options }
    }
}

impl HirDiagnosticPass for RestProtocolPass {
    fn run(&self, spec: &Specification, bag: &mut DiagnosticBag) {
        rest_hir::collect_diagnostics(spec, &self.options, bag);
    }
}

/// Validates JSON-RPC stream and attribute-collision rules across all interfaces.
pub struct JsonRpcProtocolPass;

impl HirDiagnosticPass for JsonRpcProtocolPass {
    fn run(&self, spec: &Specification, bag: &mut DiagnosticBag) {
        jsonrpc_hir::collect_diagnostics(spec, bag);
    }
}

/// Runs all source-level HIR validation passes in order.
pub(crate) struct SourceHirPasses;

impl SourceHirPasses {
    /// Runs built-in source HIR passes into `bag`.
    pub(crate) fn run(spec: &Specification, bag: &mut DiagnosticBag) {
        UnionCasePass.run(spec, bag);
        HttpUnionPass.run(spec, bag);
    }
}
