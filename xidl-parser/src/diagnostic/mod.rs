//! Unified compiler diagnostics and layered diagnostic pass traits.

mod syntax;
#[cfg(test)]
mod tests;

use serde::Serialize;
use std::ops::Range;

pub use syntax::SyntaxPass;

/// Stable identifier of a compiler diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticCode {
    /// Syntax error or missing token reported by tree-sitter.
    SyntaxError,
    /// An unknown name is within edit distance two of a built-in name.
    MisspelledAnnotation,
    /// A built-in annotation targets an unsupported declaration.
    MisplacedAnnotation,
    /// Arguments do not satisfy the annotation's contract.
    BadAnnotationArgs,
    /// An unknown annotation may be interpreted by an external plugin.
    UnknownAnnotation,
    /// Semantic validation rule failure on lowered HIR.
    SemanticError,
    /// API versioning validation failure.
    VersioningError,
    /// Protocol-specific validation failure (HTTP or JSON-RPC).
    ProtocolError,
}

impl DiagnosticCode {
    /// Stable spelling used by command-line output and editor integrations.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SyntaxError => "syntax-error",
            Self::MisspelledAnnotation => "misspelled-annotation",
            Self::MisplacedAnnotation => "misplaced-annotation",
            Self::BadAnnotationArgs => "bad-annotation-args",
            Self::UnknownAnnotation => "unknown-annotation",
            Self::SemanticError => "semantic-error",
            Self::VersioningError => "versioning-error",
            Self::ProtocolError => "protocol-error",
        }
    }
}

/// Diagnostic severity before a caller applies its warning policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The contract is invalid and blocks generation.
    Error,
    /// Advisory diagnostic that does not block generation unless denied.
    Warning,
}

/// A compiler diagnostic produced by a syntax, AST, HIR, or protocol pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Stable rule identifier.
    pub code: DiagnosticCode,
    /// Severity independent of a CLI's `--deny warnings` policy.
    pub severity: Severity,
    /// Half-open byte range in the parsed source (`0..0` when spanless).
    pub span: Range<usize>,
    /// Description of the failing contract rule.
    pub message: String,
    /// Suggested spelling or fix, when available.
    pub suggestion: Option<String>,
    /// Logical symbol location for spanless IR diagnostics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

impl Diagnostic {
    /// Creates an error diagnostic with a byte span (`0..0` when spanless).
    pub fn error(code: DiagnosticCode, span: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            span,
            message: message.into(),
            suggestion: None,
            location: None,
        }
    }

    /// Creates a warning diagnostic with a byte span (`0..0` when spanless).
    pub fn warning(code: DiagnosticCode, span: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            span,
            message: message.into(),
            suggestion: None,
            location: None,
        }
    }

    /// Attaches a suggested spelling or remediation.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Attaches a logical symbol location for spanless IR diagnostics.
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Returns `true` when this diagnostic carries a non-empty source span.
    pub fn has_span(&self) -> bool {
        !self.span.is_empty()
    }
}

/// Accumulator passed through diagnostic passes to collect multiple findings.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    /// Creates an empty diagnostic accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a single diagnostic.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Records a batch of diagnostics.
    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    /// Returns `true` when any recorded diagnostic has [`Severity::Error`].
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }

    /// Returns `true` when no diagnostics have been recorded.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Borrows all recorded diagnostics in insertion order.
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Removes duplicate diagnostics while preserving insertion order.
    pub fn dedup(&mut self) {
        let mut unique = Vec::with_capacity(self.diagnostics.len());
        for diagnostic in self.diagnostics.drain(..) {
            if !unique.contains(&diagnostic) {
                unique.push(diagnostic);
            }
        }
        self.diagnostics = unique;
    }

    /// Consumes the bag and returns diagnostics sorted by source offset.
    pub fn into_sorted(mut self) -> Vec<Diagnostic> {
        self.diagnostics
            .sort_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end));
        self.diagnostics
    }

    /// Fails with the first error message if any errors were recorded.
    pub fn check_errors(&self) -> crate::error::ParserResult<()> {
        if let Some(first) = self
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return Err(crate::error::ParseError::Message(first.message.clone()));
        }
        Ok(())
    }
}

/// A read-only diagnostic pass over a parsed [`crate::typed_ast::Specification`].
pub trait AstDiagnosticPass {
    /// Inspects `ast` and appends findings into `bag`.
    fn run(&self, ast: &crate::typed_ast::Specification, bag: &mut DiagnosticBag);
}

/// A read-only diagnostic pass over a lowered [`crate::hir::Specification`].
pub trait HirDiagnosticPass {
    /// Inspects `spec` and appends findings into `bag`.
    fn run(&self, spec: &crate::hir::Specification, bag: &mut DiagnosticBag);
}
