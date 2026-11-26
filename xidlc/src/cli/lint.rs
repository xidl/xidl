use std::path::PathBuf;

use clap::{Args, ValueEnum};
use xidl_parser::diagnostic::Severity;
use xidl_parser::error::ParseError;

use crate::diagnostic::DiagnosticDisplay;
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
                Err(ParseError::Diagnostics(diagnostics)) => {
                    for diagnostic in &diagnostics {
                        DiagnosticDisplay::new(&filename, &source, diagnostic).print();
                    }
                    failed = true;
                    continue;
                }
                Err(error) => {
                    eprintln!("{filename}: {error}");
                    failed = true;
                    continue;
                }
            };
            for diagnostic in xidl_parser::lint::lint(&ast) {
                failed |= diagnostic.severity == Severity::Error || self.deny.is_some();
                DiagnosticDisplay::new(&filename, &source, &diagnostic).print();
            }
        }
        if failed {
            Err(IdlcError::LintFailed)
        } else {
            Ok(())
        }
    }
}
