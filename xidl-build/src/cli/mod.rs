//! CLI backend that shells out to the `xidlc` executable found on `PATH`.

use crate::builder::GenerateSpec;
use crate::error::Error;
use std::io;
use std::process::{Command, Output};

/// Generator languages whose CLI form accepts `--client`, `--server`, and
/// `--mock` flags.
const CLIENT_SERVER_LANGS: [&str; 7] = [
    "rust",
    "rust-jsonrpc",
    "rust-axum",
    "typescript",
    "typescript-rest",
    "go",
    "go-rest",
];

/// Runs generation by invoking the `xidlc` CLI resolved from `PATH`.
pub(crate) struct CliBackend;

impl CliBackend {
    /// Executes `xidlc gen` for the request and maps failures to typed errors.
    pub(crate) fn run(&self, spec: &GenerateSpec) -> Result<(), Error> {
        let mut command = Self::build_command(spec)?;
        let output = command.output().map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                Error::Config(format!(
                    "xidlc executable not found on PATH ({err}); install the xidlc \
                     CLI or enable the xidl-build `bundle` feature"
                ))
            } else {
                Error::Io(err)
            }
        })?;
        if output.status.success() {
            Ok(())
        } else {
            Err(Error::Compile(Self::failure_message(&output)))
        }
    }

    /// Builds the `xidlc gen` invocation for the request.
    fn build_command(spec: &GenerateSpec) -> Result<Command, Error> {
        let client_server_lang = CLIENT_SERVER_LANGS.contains(&spec.lang.as_str());
        if client_server_lang && !spec.client && !spec.server {
            return Err(Error::Config(format!(
                "xidlc CLI cannot express client=false and server=false for the \
                 '{}' generator; enable at least one side or use the xidl-build \
                 `bundle` feature",
                spec.lang
            )));
        }
        if !client_server_lang && spec.mock {
            return Err(Error::Config(format!(
                "xidlc CLI does not accept --mock for the '{}' generator",
                spec.lang
            )));
        }

        let mut command = Command::new("xidlc");
        command.arg("gen");
        command.arg("--out-dir").arg(&spec.out_dir);
        command.arg(&spec.lang);
        if client_server_lang {
            if spec.client {
                command.arg("--client");
            }
            if spec.server {
                command.arg("--server");
            }
            if spec.mock {
                command.arg("--mock");
            }
        }
        command.args(&spec.inputs);
        Ok(command)
    }

    /// Renders compiler stderr (or stdout) into the reported compile error.
    fn failure_message(output: &Output) -> String {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = match (stderr.trim(), stdout.trim()) {
            ("", "") => String::new(),
            ("", stdout) => stdout.to_string(),
            (stderr, _) => stderr.to_string(),
        };
        if detail.is_empty() {
            format!("xidlc CLI failed with {}", output.status)
        } else {
            format!("xidlc CLI failed with {}:\n{detail}", output.status)
        }
    }
}

#[cfg(test)]
mod tests;
