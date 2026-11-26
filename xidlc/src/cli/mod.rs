mod fmt;
mod generate;
mod lint;
mod run;
mod version;

use crate::driver;
use crate::error::IdlcResult;
use crate::library::{LibraryLoader, Toolchain};
use clap::{Args, Parser, Subcommand};
#[cfg(test)]
use generate::GenLang;
use generate::{ArgsGen, FilesArgs};
use std::ffi::OsString;
use std::path::PathBuf;

/// Top-level command-line interface for `xidlc`.
#[derive(Debug, Parser)]
#[command(name = "xidlc", about = "XIDL Compiler", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check annotation names, placement and arguments without generating files.
    Lint(lint::ArgsLint),
    Gen(ArgsGen),
    Version(version::ArgsVersion),
    #[command(alias = "format")]
    Fmt(fmt::ArgsFormat),
    #[command(name = "import")]
    Import(ArgsImport),
}

#[derive(Debug, Args)]
struct ArgsImport {
    #[arg(long = "out-dir", short = 'o', default_value = ".")]
    out_dir: String,
    #[command(subcommand)]
    lang: ImportLang,
}

#[derive(Debug, Subcommand)]
enum ImportLang {
    #[command(name = "openapi")]
    Openapi(FilesArgs),
}

/// Re-runs this command with a cached `xidlc` when a manifest demands one.
///
/// A `compiler` range the running binary satisfies keeps the current process;
/// otherwise the matching release is installed into the toolchain cache and
/// executed with the same arguments, mirroring its exit status.
fn switch_toolchain(files: &[PathBuf]) -> IdlcResult<()> {
    let manifests = LibraryLoader::new().discover(files)?;
    let toolchain = Toolchain::new();
    for loaded in manifests {
        let Some(requirement) = loaded.manifest().compiler() else {
            continue;
        };
        if let Some(binary) = toolchain.ensure(requirement)? {
            return exec_toolchain(&binary);
        }
    }
    Ok(())
}

/// Executes `binary` with this command's arguments and mirrors its exit code.
fn exec_toolchain(binary: &std::path::Path) -> IdlcResult<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<OsString>>();
    let status = std::process::Command::new(binary).args(args).status()?;
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests;
