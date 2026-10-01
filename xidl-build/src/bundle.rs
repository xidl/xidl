//! Bundle backend that runs the `xidlc` compiler in-process.

use crate::builder::GenerateSpec;
use crate::error::Error;
use xidlc::error::{DiagnosticError, DiagnosticListError, IdlcError};

/// Runs generation by linking the `xidlc` compiler driver into this process.
pub(crate) struct BundleBackend;

impl BundleBackend {
    /// Executes the compiler driver for the request and maps errors.
    pub(crate) fn run(&self, spec: &GenerateSpec) -> Result<(), Error> {
        let args = xidlc::driver::ArgsGenerate {
            lang: spec.lang.clone(),
            out_dir: spec.out_dir.to_string_lossy().into_owned(),
            client: spec.client,
            server: spec.server,
            mock: spec.mock,
            dry_run: false,
            files: spec.inputs.clone(),
        };
        xidlc::driver::Driver::run(args).map_err(Self::map_error)
    }

    /// Preserves IO failures and flattens compiler failures into messages.
    fn map_error(err: IdlcError) -> Error {
        match err {
            IdlcError::Io(io_err) => Error::Io(io_err),
            IdlcError::Diagnostics(DiagnosticListError { diagnostics }) => {
                Error::Compile(Self::format_diagnostics(&diagnostics))
            }
            other => Error::Compile(other.to_string()),
        }
    }

    /// Formats compiler diagnostics as `filename: message (label)` lines.
    fn format_diagnostics(diagnostics: &[DiagnosticError]) -> String {
        diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}: {} ({})",
                    diagnostic.filename, diagnostic.message, diagnostic.label
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
