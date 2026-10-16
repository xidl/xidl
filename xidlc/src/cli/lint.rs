use std::path::PathBuf;

use clap::{Args, ValueEnum};
use xidl_parser::lint::{Diagnostic, Severity};

use crate::error::{IdlcError, IdlcResult};

#[derive(Debug, Args)]
pub(super) struct ArgsLint {
    /// Treat diagnostics of this severity as errors.
    #[arg(long, value_enum)]
    deny: Option<Deny>,
    /// IDL source files to check.
    #[arg(required = true, num_args = 1..)]
    files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Deny {
    Warnings,
}

impl ArgsLint {
    pub(super) fn execute(self) -> IdlcResult<()> {
        let mut failed = false;
        for file in self.files {
            let filename = file.display().to_string();
            let source = match std::fs::read_to_string(&file) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!("{filename}: {error}");
                    failed = true;
                    continue;
                }
            };
            let ast = match xidl_parser::parser::parser_text(&source) {
                Ok(ast) => ast,
                Err(error) => {
                    eprintln!("{filename}: {error}");
                    failed = true;
                    continue;
                }
            };
            for diagnostic in xidl_parser::lint::lint(&ast) {
                failed |= diagnostic.severity == Severity::Error || self.deny.is_some();
                LintDisplay {
                    filename: &filename,
                    source: &source,
                    diagnostic: &diagnostic,
                }
                .print();
            }
        }
        if failed {
            Err(IdlcError::LintFailed)
        } else {
            Ok(())
        }
    }
}

struct LintDisplay<'a> {
    filename: &'a str,
    source: &'a str,
    diagnostic: &'a Diagnostic,
}

impl LintDisplay<'_> {
    fn print(&self) {
        let diagnostic = self.diagnostic;
        let prefix = &self.source[..diagnostic.span.start];
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
