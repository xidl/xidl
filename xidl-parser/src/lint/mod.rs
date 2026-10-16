//! Pure annotation diagnostics and the built-in annotation vocabulary.
//!
//! Generation consumes the vocabulary, never the lint pass. Editors can use
//! `BUILTIN_ANNOTATIONS` for completion and `lint` for source diagnostics.

mod arguments;
mod check;
mod registry;
mod walk;

pub use check::{Diagnostic, DiagnosticCode, Severity};
pub use registry::{
    AnnotationDefinition, AnnotationKind, ArgumentShape, BUILTIN_ANNOTATIONS, Site,
};

/// Checks annotations in a parsed typed AST without performing IO or lowering it.
pub fn lint(ast: &crate::typed_ast::Specification) -> Vec<Diagnostic> {
    let mut checker = check::Checker::default();
    checker.definitions(&ast.0);
    checker.finish()
}
