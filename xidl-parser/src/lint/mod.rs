//! Pure annotation diagnostics and the built-in annotation vocabulary.
//!
//! Generation can validate selected contracts without enabling the lint pass. Editors use
//! `BUILTIN_ANNOTATIONS` for completion and `lint` for source diagnostics.

mod arguments;
mod check;
mod registry;
mod walk;

use crate::diagnostic::{AstDiagnosticPass, DiagnosticBag};

pub use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};
pub use registry::{
    AnnotationDefinition, AnnotationKind, ArgumentShape, BUILTIN_ANNOTATIONS, Site,
};

/// AST diagnostic pass validating built-in annotation names, placement, and arguments.
#[derive(Debug, Default, Clone, Copy)]
pub struct AnnotationLintPass {
    selected: Option<AnnotationKind>,
}

impl AnnotationLintPass {
    /// Creates a pass checking all built-in annotations and unknown warnings.
    pub fn all() -> Self {
        Self { selected: None }
    }

    /// Creates a pass validating only `kind` before lowering loses source spans.
    pub fn for_kind(kind: AnnotationKind) -> Self {
        Self {
            selected: Some(kind),
        }
    }
}

impl AstDiagnosticPass for AnnotationLintPass {
    fn run(&self, ast: &crate::typed_ast::Specification, bag: &mut DiagnosticBag) {
        let mut checker = match self.selected {
            Some(kind) => check::Checker::for_annotation(kind),
            None => check::Checker::default(),
        };
        checker.definitions(&ast.0);
        bag.extend(checker.finish());
    }
}

/// Checks annotations in a parsed typed AST without performing IO or lowering it.
pub fn lint(ast: &crate::typed_ast::Specification) -> Vec<Diagnostic> {
    let mut bag = DiagnosticBag::new();
    AnnotationLintPass::all().run(ast, &mut bag);
    bag.into_sorted()
}
