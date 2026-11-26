#[cfg(feature = "cli")]
mod colors;
#[cfg(feature = "cli")]
mod theme;

#[cfg(feature = "cli")]
mod highlight;
#[cfg(feature = "cli")]
pub use highlight::TreeSitterMietteHighlighter;

use crate::error::{DiagnosticError, IdlcError, IdlcResult};
use xidl_parser::diagnostic::{Diagnostic, Severity, SyntaxPass};

/// Runner for IDL syntax diagnostics backed by [`SyntaxPass`].
pub struct DiagnosticRunner;

impl DiagnosticRunner {
    /// Creates a runner for IDL syntax diagnostics.
    pub fn new_idl() -> Self {
        Self
    }

    /// Runs the parser's [`SyntaxPass`] on `source`, returning miette-ready diagnostics on error.
    pub fn run(&self, source: &str, filename: &str) -> IdlcResult<()> {
        let diagnostics = SyntaxPass::check_source(source)?;
        if diagnostics.is_empty() {
            return Ok(());
        }
        let errors = diagnostics
            .into_iter()
            .map(|diagnostic| {
                let offset = diagnostic.span.start;
                let len = diagnostic.span.end.saturating_sub(offset);
                DiagnosticError::from_span(filename, source, offset, len, diagnostic.message)
            })
            .collect();
        Err(IdlcError::diagnostics(errors))
    }
}

/// Formats [`Diagnostic`] items consistently across CLI commands.
pub struct DiagnosticDisplay<'a> {
    filename: &'a str,
    source: &'a str,
    diagnostic: &'a Diagnostic,
}

impl<'a> DiagnosticDisplay<'a> {
    /// Creates a display formatter for one source-located diagnostic.
    pub fn new(filename: &'a str, source: &'a str, diagnostic: &'a Diagnostic) -> Self {
        Self {
            filename,
            source,
            diagnostic,
        }
    }

    /// Prints this diagnostic to stderr with line/column location and suggestion.
    pub fn print(&self) {
        let diagnostic = self.diagnostic;
        let offset = diagnostic.span.start.min(self.source.len());
        let prefix = &self.source[..offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit('\n')
            .next()
            .map_or(1, |line| line.chars().count() + 1);
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        eprintln!(
            "{}:{line}:{column}: {severity}[{}]: {}",
            self.filename,
            diagnostic.code.as_str(),
            diagnostic.message
        );
        if let Some(suggestion) = &diagnostic.suggestion {
            eprintln!("  did you mean `@{suggestion}`?");
        }
    }
}
