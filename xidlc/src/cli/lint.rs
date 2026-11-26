use std::path::PathBuf;

use clap::{Args, ValueEnum};
use xidl_parser::diagnostic::{AstDiagnosticPass, DiagnosticBag, Severity};
use xidl_parser::error::ParseError;
use xidl_parser::hir::FsIncludeResolver;
use xidl_parser::lint::AnnotationLintPass;
use xidl_parser::pipeline::{DiagnoseOptions, HirProjectionKind, Pipeline};

use crate::diagnostic::DiagnosticDisplay;
use crate::error::{IdlcError, IdlcResult};

#[derive(Debug, Args)]
pub(super) struct ArgsLint {
    /// Treat diagnostics of this severity as errors.
    #[arg(long, value_enum)]
    deny: Option<Deny>,
    /// Also run semantic, versioning, and protocol diagnostic passes on complete contracts.
    #[arg(long, default_value_t = false)]
    full: bool,
    /// Protocol rule set to validate (implies `--full`).
    #[arg(long, value_enum)]
    protocol: Option<LintProtocol>,
    /// IDL source files to check.
    #[arg(required = true, num_args = 1..)]
    files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Deny {
    Warnings,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LintProtocol {
    Http,
    Jsonrpc,
    Rpc,
}

impl LintProtocol {
    fn into_kind(self) -> HirProjectionKind {
        match self {
            Self::Http => HirProjectionKind::Http,
            Self::Jsonrpc => HirProjectionKind::JsonRpc,
            Self::Rpc => HirProjectionKind::Rpc,
        }
    }
}

impl ArgsLint {
    pub(super) fn execute(self) -> IdlcResult<()> {
        let mut failed = false;
        let mut resolver = FsIncludeResolver;
        let run_full = self.full || self.protocol.is_some();
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
            let diagnostics = if run_full {
                let mut options = DiagnoseOptions::new(&file);
                if let Some(protocol) = self.protocol {
                    options = options.with_protocol(protocol.into_kind());
                }
                Pipeline::diagnose_source(&source, options, &mut resolver)
            } else {
                match Self::lint_fragment(&source) {
                    Ok(bag) => bag,
                    Err(error) => {
                        eprintln!("{filename}: {error}");
                        failed = true;
                        continue;
                    }
                }
            };
            for diagnostic in diagnostics.as_slice() {
                failed |= diagnostic.severity == Severity::Error || self.deny.is_some();
                DiagnosticDisplay::new(&filename, &source, diagnostic).print();
            }
        }
        if failed {
            Err(IdlcError::LintFailed)
        } else {
            Ok(())
        }
    }

    fn lint_fragment(source: &str) -> Result<DiagnosticBag, ParseError> {
        let mut bag = DiagnosticBag::new();
        match xidl_parser::parser::parser_text(source) {
            Ok(ast) => {
                AnnotationLintPass::all().run(&ast, &mut bag);
                Ok(bag)
            }
            Err(ParseError::Diagnostics(diagnostics)) => {
                bag.extend(diagnostics);
                Ok(bag)
            }
            Err(error) => Err(error),
        }
    }
}
