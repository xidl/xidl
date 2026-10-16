use super::error::LibraryError;
use std::process::Command;

/// Runs the external commands that materialize downloaded artifacts.
///
/// Dependency packages and compiler toolchains are fetched with `git`, `curl`
/// and `tar`, the same tools the repository's own release tooling uses, so the
/// compiler keeps no HTTP client of its own.
pub struct CommandRunner;

impl CommandRunner {
    /// Runs `program` and captures its standard output.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Command`] when the program is missing, exits
    /// non-zero, or writes output that is not valid UTF-8.
    pub fn output(program: &str, args: &[String]) -> Result<String, LibraryError> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|err| Self::error(program, args, err.to_string()))?;
        if !output.status.success() {
            return Err(Self::error(
                program,
                args,
                failure_detail(&output.stdout, &output.stderr),
            ));
        }
        String::from_utf8(output.stdout).map_err(|err| Self::error(program, args, err.to_string()))
    }

    /// Runs `program` and inherits the caller's standard streams.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Command`] when the program is missing or exits
    /// non-zero.
    pub fn run(program: &str, args: &[String]) -> Result<(), LibraryError> {
        let status = Command::new(program)
            .args(args)
            .status()
            .map_err(|err| Self::error(program, args, err.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            Err(Self::error(program, args, status.to_string()))
        }
    }

    /// Builds the error reported for a failed external command.
    fn error(program: &str, args: &[String], reason: impl Into<String>) -> LibraryError {
        LibraryError::Command {
            program: program.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            reason: reason.into(),
        }
    }
}

/// Renders the interesting part of a failed command's captured streams.
fn failure_detail(stdout: &[u8], stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let stdout = String::from_utf8_lossy(stdout);
    match (stderr.trim(), stdout.trim()) {
        ("", "") => "exited with a non-zero status".to_string(),
        ("", stdout) => stdout.to_string(),
        (stderr, _) => stderr.to_string(),
    }
}

/// Converts a path into a UTF-8 argument for an external command.
pub(super) fn argument(path: &std::path::Path) -> String {
    path.as_os_str().to_string_lossy().into_owned()
}
