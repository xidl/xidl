//! Diagnostic passes executed by the parser compilation pipeline.

use crate::diagnostic::{
    AstDiagnosticPass, Diagnostic, DiagnosticBag, DiagnosticCode, HirDiagnosticPass,
};
use crate::hir::Specification;
use crate::lint::{AnnotationKind, AnnotationLintPass};
use crate::rest_hir::HttpUnion;
use crate::semantic::UnionCaseLabels;
use crate::typed_ast;

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
        if let Err(err) = UnionCaseLabels::validate(&spec.0) {
            bag.push(Diagnostic::error(
                DiagnosticCode::SemanticError,
                0..0,
                err.to_string(),
            ));
        }
    }
}

/// Validates HTTP union declarations on a lowered specification.
pub struct HttpUnionPass;

impl HirDiagnosticPass for HttpUnionPass {
    fn run(&self, spec: &Specification, bag: &mut DiagnosticBag) {
        if let Err(err) = HttpUnion::validate_source(&spec.0) {
            bag.push(Diagnostic::error(
                DiagnosticCode::ProtocolError,
                0..0,
                err.to_string(),
            ));
        }
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
