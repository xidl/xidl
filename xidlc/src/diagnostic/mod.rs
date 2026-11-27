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
use xidl_parser::error::ParseError;

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
        Err(self.from_diagnostics(source, filename, diagnostics))
    }

    /// Converts a parser failure, preserving structured diagnostics.
    pub fn from_parse_error(&self, source: &str, filename: &str, error: ParseError) -> IdlcError {
        match error {
            ParseError::Diagnostics(diagnostics) => {
                self.from_diagnostics(source, filename, diagnostics)
            }
            error => IdlcError::Parse(error),
        }
    }

    /// Converts parser diagnostics into the compiler's miette-ready error.
    pub fn from_diagnostics(
        &self,
        source: &str,
        filename: &str,
        diagnostics: Vec<Diagnostic>,
    ) -> IdlcError {
        let errors = diagnostics
            .into_iter()
            .map(|diagnostic| {
                let offset = diagnostic.span.start;
                let len = diagnostic.span.end.saturating_sub(offset);
                DiagnosticError::from_span(filename, source, offset, len, diagnostic.message)
            })
            .collect();
        IdlcError::diagnostics(errors)
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

    /// Prints this diagnostic to stderr with source or symbol location and suggestion.
    pub fn print(&self) {
        let diagnostic = self.diagnostic;
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        if diagnostic.has_span() {
            let offset = diagnostic.span.start.min(self.source.len());
            let prefix = &self.source[..offset];
            let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let column = prefix
                .rsplit('\n')
                .next()
                .map_or(1, |line| line.chars().count() + 1);
            eprintln!(
                "{}:{line}:{column}: {severity}[{}]: {}",
                self.filename,
                diagnostic.code.as_str(),
                diagnostic.message
            );
        } else if let Some(location) = diagnostic.location.as_deref().filter(|loc| !loc.is_empty())
        {
            eprintln!(
                "{} ({location}): {severity}[{}]: {}",
                self.filename,
                diagnostic.code.as_str(),
                diagnostic.message
            );
        } else {
            eprintln!(
                "{}: {severity}[{}]: {}",
                self.filename,
                diagnostic.code.as_str(),
                diagnostic.message
            );
        }
        if let Some(suggestion) = &diagnostic.suggestion {
            eprintln!("  did you mean `@{suggestion}`?");
        }
    }
}
