use crate::error::{IdlcError, IdlcResult};
use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(super) struct ArgsVersion {
    #[command(subcommand)]
    pub(super) command: VersionCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum VersionCommand {
    Diff(ArgsVersionDiff),
    /// Write or extend a version lock file with stable per-version fingerprints.
    Lock(ArgsVersionLock),
    /// Verify that frozen versions in the lock file have not changed.
    Check(ArgsVersionCheck),
}

#[derive(Debug, Args)]
pub(super) struct ArgsVersionDiff {
    /// Earlier version member name or display value.
    pub(super) from: String,
    /// Later version member name or display value.
    pub(super) to: String,
    /// Versioned IDL contract.
    pub(super) idl: PathBuf,
}

#[derive(Debug, Args)]
struct ArgsSelectedVersions {
    /// Limit the command to these version values. Repeat to select several.
    #[arg(long = "version", value_name = "VALUE", value_parser = NonEmptyStringValueParser::new())]
    values: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct ArgsVersionLock {
    /// Lock file path (defaults to `<idl_stem>.versions.lock.json` next to the IDL file).
    #[arg(long = "lock-file", short = 'l', value_name = "PATH")]
    pub(super) lock_file: Option<PathBuf>,
    /// Overwrite existing locked fingerprints when a frozen version changed.
    #[arg(long = "force", default_value_t = false)]
    pub(super) force: bool,
    #[command(flatten)]
    versions: ArgsSelectedVersions,
    /// Versioned IDL contract.
    pub(super) idl: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct ArgsVersionCheck {
    /// Lock file path (defaults to `<idl_stem>.versions.lock.json` next to the IDL file).
    #[arg(long = "lock-file", short = 'l', value_name = "PATH")]
    pub(super) lock_file: Option<PathBuf>,
    /// Also fail when the contract declares versions missing from the lock file.
    #[arg(long = "strict", default_value_t = false)]
    pub(super) strict: bool,
    #[command(flatten)]
    versions: ArgsSelectedVersions,
    /// Versioned IDL contract.
    pub(super) idl: PathBuf,
}

impl ArgsVersion {
    /// Executes the selected `xidlc version` subcommand.
    pub(super) fn execute(self) -> IdlcResult<()> {
        match self.command {
            VersionCommand::Diff(args) => Self::run_diff(args),
            VersionCommand::Lock(args) => Self::run_lock(args),
            VersionCommand::Check(args) => Self::run_check(args),
        }
    }

    fn run_diff(args: ArgsVersionDiff) -> IdlcResult<()> {
        match crate::version_diff::VersionDiff::from_file(&args.idl)
            .and_then(|diff| diff.compare(&args.from, &args.to))
        {
            Ok(Some(diff)) => {
                let mut output = std::io::stdout().lock();
                output
                    .write_all(diff.as_bytes())
                    .and_then(|_| output.flush())
                    .map_err(|error| IdlcError::CliExit {
                        code: 2,
                        message: Some(format!(
                            "version diff: cannot write output for '{}': {error}",
                            args.idl.display()
                        )),
                    })?;
                Err(IdlcError::CliExit {
                    code: 1,
                    message: None,
                })
            }
            Ok(None) => Ok(()),
            Err(error) => Err(IdlcError::CliExit {
                code: 2,
                message: Some(format!(
                    "version diff: contract '{}': {error}",
                    args.idl.display()
                )),
            }),
        }
    }

    fn run_lock(args: ArgsVersionLock) -> IdlcResult<()> {
        let lock_path = args
            .lock_file
            .unwrap_or_else(|| crate::version_lock::VersionLock::default_lock_path(&args.idl));
        match crate::version_lock::VersionLock::from_file(&args.idl).and_then(|lock| {
            lock.with_versions(args.versions.values)
                .write_lock(&lock_path, args.force)
        }) {
            Ok(report) if report.is_written() => Ok(()),
            Ok(report) => Err(IdlcError::CliExit {
                code: 1,
                message: Some(report.render_drifts(&args.idl).trim_end().to_string()),
            }),
            Err(error) => Err(IdlcError::CliExit {
                code: 2,
                message: Some(format!(
                    "version lock: contract '{}': {error}",
                    args.idl.display()
                )),
            }),
        }
    }

    fn run_check(args: ArgsVersionCheck) -> IdlcResult<()> {
        let lock_path = args
            .lock_file
            .unwrap_or_else(|| crate::version_lock::VersionLock::default_lock_path(&args.idl));
        match crate::version_lock::VersionLock::from_file(&args.idl).and_then(|lock| {
            lock.with_versions(args.versions.values)
                .check(&lock_path, args.strict)
        }) {
            Ok(report) if report.is_clean() => Ok(()),
            Ok(report) => Err(IdlcError::CliExit {
                code: 1,
                message: Some(report.render(&args.idl, &lock_path).trim_end().to_string()),
            }),
            Err(error) => Err(IdlcError::CliExit {
                code: 2,
                message: Some(format!(
                    "version check: contract '{}': {error}",
                    args.idl.display()
                )),
            }),
        }
    }
}
