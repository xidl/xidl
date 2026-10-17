mod fmt;
mod lint;

use crate::driver;
use crate::driver::ArgsGenerate;
use crate::error::{IdlcError, IdlcResult};
use crate::library::{LibraryLoader, Toolchain};
use clap::{Args, CommandFactory, Parser, Subcommand};
use std::ffi::OsString;
use std::path::PathBuf;

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

#[derive(Debug, Args)]
struct ArgsGen {
    #[arg(long = "out-dir", short = 'o', default_value = ".", global = true)]
    out_dir: String,
    #[arg(long = "dry-run", default_value_t = false, global = true)]
    dry_run: bool,
    /// Generate only this API version (version enum member name or value).
    #[arg(
        long = "api-version",
        value_name = "VERSION",
        conflicts_with = "api_versions",
        global = true
    )]
    api_version: Option<String>,
    /// Generate these API versions, comma separated, or `*` for every version.
    #[arg(long = "api-versions", value_name = "VERSIONS", global = true)]
    api_versions: Option<String>,
    #[command(subcommand)]
    lang: GenLang,
}

#[derive(Debug, Subcommand)]
enum GenLang {
    #[command(name = "hir", hide = true)]
    Hir(FilesArgs),
    #[command(name = "rest-hir", alias = "rest_hir", hide = true)]
    RestHir(FilesArgs),
    #[command(name = "jsonrpc-hir", alias = "jsonrpc_hir", hide = true)]
    JsonRpcHir(FilesArgs),
    #[command(name = "typed-ast", alias = "typed_ast", hide = true)]
    TypedAst(FilesArgs),
    #[command(name = "rust", alias = "rs")]
    Rust(ClientServerArgs),
    #[command(
        name = "rust-jsonrpc",
        alias = "rust_jsonrpc",
        alias = "rs-jsonrpc",
        alias = "rs_jsonrpc"
    )]
    RustJsonRpc(ClientServerArgs),
    #[command(
        name = "rust-axum",
        alias = "rust_axum",
        alias = "rs-axum",
        alias = "rs_axum",
        alias = "axum"
    )]
    RustAxum(ClientServerArgs),
    #[command(name = "typescript", alias = "ts")]
    Typescript(ClientServerArgs),
    #[command(
        name = "typescript-rest",
        alias = "typescript_rest",
        alias = "ts-rest",
        alias = "ts_rest"
    )]
    TypescriptRest(ClientServerArgs),
    #[command(name = "go", alias = "golang")]
    Go(ClientServerArgs),
    #[command(name = "go-rest", alias = "go_rest")]
    GoRest(ClientServerArgs),
    #[command(name = "openapi")]
    Openapi(OpenApiArgs),
    #[command(name = "openrpc", alias = "open-rpc")]
    Openrpc(FilesArgs),
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Debug, Args)]
struct FilesArgs {
    files: Vec<PathBuf>,
}

/// Generates an OpenAPI JSON document for HTTP interfaces.
#[derive(Debug, Args)]
struct OpenApiArgs {
    /// OpenAPI document version. `auto` keeps the generator's choice (OpenAPI
    /// 3.1, upgraded to 3.2 when the contract has streams); `3.1` forces an
    /// OpenAPI 3.1 document, where streams are rendered best-effort.
    #[arg(long = "version", value_name = "VERSION", default_value = "auto")]
    version: OpenApiVersion,
    files: Vec<PathBuf>,
}

/// Accepted `--version` values for the `openapi` target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum OpenApiVersion {
    /// Let the generator choose the document version.
    Auto,
    /// Force an OpenAPI 3.1 document.
    #[value(name = "3.1")]
    V31,
}

impl OpenApiVersion {
    /// Generator property value, or `None` to keep the `auto` behaviour.
    fn property(self) -> Option<String> {
        match self {
            Self::Auto => None,
            Self::V31 => Some("3.1".to_string()),
        }
    }
}

#[derive(Debug, Args)]
struct ClientServerArgs {
    #[arg(long = "client", default_value_t = false)]
    client: bool,
    #[arg(long = "server", default_value_t = false)]
    server: bool,
    #[arg(long = "mock", default_value_t = false)]
    mock: bool,
    files: Vec<PathBuf>,
}

#[derive(Debug)]
struct SharedGenArgs {
    out_dir: String,
    dry_run: bool,
    api_version: Option<String>,
    api_versions: Option<String>,
}

impl ArgsGen {
    fn into_driver_args(self) -> IdlcResult<ArgsGenerate> {
        let shared = SharedGenArgs {
            out_dir: self.out_dir,
            dry_run: self.dry_run,
            api_version: self.api_version,
            api_versions: self.api_versions,
        };
        self.lang.into_driver_args(shared)
    }
}

impl GenLang {
    fn into_driver_args(self, shared: SharedGenArgs) -> IdlcResult<ArgsGenerate> {
        match self {
            Self::Hir(args) => Ok(shared.into_plain("hir", args.files)),
            Self::RestHir(args) => Ok(shared.into_plain("rest-hir", args.files)),
            Self::JsonRpcHir(args) => Ok(shared.into_plain("jsonrpc-hir", args.files)),
            Self::TypedAst(args) => Ok(shared.into_plain("typed-ast", args.files)),
            Self::Rust(args) => Ok(shared.into_client_server("rust", args)),
            Self::RustJsonRpc(args) => Ok(shared.into_client_server("rust-jsonrpc", args)),
            Self::RustAxum(args) => Ok(shared.into_client_server("rust-axum", args)),
            Self::Typescript(args) => Ok(shared.into_client_server("typescript", args)),
            Self::TypescriptRest(args) => Ok(shared.into_client_server("typescript-rest", args)),
            Self::Go(args) => Ok(shared.into_client_server("go", args)),
            Self::GoRest(args) => Ok(shared.into_client_server("go-rest", args)),
            Self::Openapi(args) => {
                let mut generate = shared.into_plain("openapi", args.files);
                generate.openapi_version = args.version.property();
                Ok(generate)
            }
            Self::Openrpc(args) => Ok(shared.into_plain("openrpc", args.files)),
            Self::External(values) => parse_external(shared, values),
        }
    }

    fn help_command(&self) -> clap::Command {
        let usage = self.usage();
        let mut command = Cli::command();
        let Some(gen_cmd) = command.find_subcommand_mut("gen") else {
            return command;
        };
        let subcommand = match self {
            Self::Hir(_) => gen_cmd.find_subcommand_mut("hir"),
            Self::RestHir(_) => gen_cmd.find_subcommand_mut("rest-hir"),
            Self::JsonRpcHir(_) => gen_cmd.find_subcommand_mut("jsonrpc-hir"),
            Self::TypedAst(_) => gen_cmd.find_subcommand_mut("typed-ast"),
            Self::Rust(_) => gen_cmd.find_subcommand_mut("rust"),
            Self::RustJsonRpc(_) => gen_cmd.find_subcommand_mut("rust-jsonrpc"),
            Self::RustAxum(_) => gen_cmd.find_subcommand_mut("rust-axum"),
            Self::Typescript(_) => gen_cmd.find_subcommand_mut("typescript"),
            Self::TypescriptRest(_) => gen_cmd.find_subcommand_mut("typescript-rest"),
            Self::Go(_) => gen_cmd.find_subcommand_mut("go"),
            Self::GoRest(_) => gen_cmd.find_subcommand_mut("go-rest"),
            Self::Openapi(_) => gen_cmd.find_subcommand_mut("openapi"),
            Self::Openrpc(_) => gen_cmd.find_subcommand_mut("openrpc"),
            Self::External(_) => None,
        };
        if let Some(subcommand) = subcommand {
            subcommand.clone().override_usage(usage)
        } else {
            command
        }
    }

    fn usage(&self) -> &'static str {
        match self {
            Self::Hir(_) => "xidlc gen hir [FILES]...",
            Self::RestHir(_) => "xidlc gen rest-hir [FILES]...",
            Self::JsonRpcHir(_) => "xidlc gen jsonrpc-hir [FILES]...",
            Self::TypedAst(_) => "xidlc gen typed-ast [FILES]...",
            Self::Rust(_) => "xidlc gen rust [OPTIONS] [FILES]...",
            Self::RustJsonRpc(_) => "xidlc gen rust-jsonrpc [OPTIONS] [FILES]...",
            Self::RustAxum(_) => "xidlc gen rust-axum [OPTIONS] [FILES]...",
            Self::Typescript(_) => "xidlc gen typescript [OPTIONS] [FILES]...",
            Self::TypescriptRest(_) => "xidlc gen typescript-rest [OPTIONS] [FILES]...",
            Self::Go(_) => "xidlc gen go [OPTIONS] [FILES]...",
            Self::GoRest(_) => "xidlc gen go-rest [OPTIONS] [FILES]...",
            Self::Openapi(_) => "xidlc gen openapi [OPTIONS] [FILES]...",
            Self::Openrpc(_) => "xidlc gen openrpc [FILES]...",
            Self::External(_) => "xidlc gen <lang> [FILES]...",
        }
    }
}

impl SharedGenArgs {
    fn into_plain(self, lang: impl Into<String>, files: Vec<PathBuf>) -> ArgsGenerate {
        ArgsGenerate {
            lang: lang.into(),
            out_dir: self.out_dir,
            client: false,
            server: true,
            dry_run: self.dry_run,
            mock: false,
            api_version: self.api_version,
            api_versions: self.api_versions,
            openapi_version: None,
            files,
        }
    }

    fn into_client_server(self, lang: impl Into<String>, args: ClientServerArgs) -> ArgsGenerate {
        let lang = lang.into();
        let (client, server) = if !args.client && !args.server {
            if lang == "typescript-rest" {
                (true, false)
            } else {
                (false, true)
            }
        } else {
            (args.client, args.server)
        };
        ArgsGenerate {
            lang,
            out_dir: self.out_dir,
            client,
            server,
            mock: args.mock,
            dry_run: self.dry_run,
            api_version: self.api_version,
            api_versions: self.api_versions,
            openapi_version: None,
            files: args.files,
        }
    }
}

fn parse_external(shared: SharedGenArgs, values: Vec<OsString>) -> IdlcResult<ArgsGenerate> {
    let mut values = values.into_iter();
    let Some(lang) = values.next() else {
        return Err(IdlcError::fmt("missing generator language"));
    };
    let Some(lang) = lang.to_str() else {
        return Err(IdlcError::fmt("invalid utf-8 generator language"));
    };
    let files = values.map(PathBuf::from).collect::<Vec<_>>();
    Ok(shared.into_plain(lang, files))
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

impl Cli {
    pub fn run(self) -> IdlcResult<()> {
        match self.command {
            Command::Lint(args) => args.execute(),
            Command::Gen(args) => {
                let help_command = args.lang.help_command();
                let args = args.into_driver_args()?;
                if args.files.is_empty() {
                    help_command.clone().print_help()?;
                    println!();
                    return Ok(());
                }
                switch_toolchain(&args.files)?;
                driver::Driver::run(args)
            }
            Command::Fmt(args) => args.execute(),
            Command::Import(args) => {
                let out_dir = PathBuf::from(&args.out_dir);
                if !out_dir.exists() {
                    std::fs::create_dir_all(&out_dir)?;
                }
                match args.lang {
                    ImportLang::Openapi(files_args) => {
                        for file in files_args.files {
                            crate::import::openapi::import_openapi(&file, &out_dir)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests;
