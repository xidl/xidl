use crate::cli::Cli;
use crate::driver::ArgsGenerate;
use crate::error::{IdlcError, IdlcResult};
use clap::{Args, CommandFactory, Subcommand};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(super) struct ArgsGen {
    #[arg(long = "out-dir", short = 'o', default_value = ".", global = true)]
    pub(super) out_dir: String,
    #[arg(long = "dry-run", default_value_t = false, global = true)]
    pub(super) dry_run: bool,
    /// Generate only this API version (version enum member name or value).
    #[arg(
        long = "api-version",
        value_name = "VERSION",
        conflicts_with = "api_versions",
        global = true
    )]
    pub(super) api_version: Option<String>,
    /// Generate these API versions, comma separated, or `*` for every version.
    #[arg(long = "api-versions", value_name = "VERSIONS", global = true)]
    pub(super) api_versions: Option<String>,
    #[command(subcommand)]
    pub(super) lang: GenLang,
}

#[derive(Debug, Subcommand)]
pub(super) enum GenLang {
    #[command(name = "hir", hide = true)]
    Hir(FilesArgs),
    #[command(name = "rest-hir", alias = "rest_hir", hide = true)]
    RestHir(FilesArgs),
    #[command(name = "jsonrpc-hir", alias = "jsonrpc_hir", hide = true)]
    JsonRpcHir(FilesArgs),
    #[command(name = "typed-ast", alias = "typed_ast", hide = true)]
    TypedAst(FilesArgs),
    #[cfg(feature = "gen-rust")]
    #[command(name = "rust", alias = "rs")]
    Rust(ClientServerArgs),
    #[cfg(feature = "gen-rust-jsonrpc")]
    #[command(
        name = "rust-jsonrpc",
        alias = "rust_jsonrpc",
        alias = "rs-jsonrpc",
        alias = "rs_jsonrpc"
    )]
    RustJsonRpc(ClientServerArgs),
    #[cfg(feature = "gen-rust-axum")]
    #[command(
        name = "rust-axum",
        alias = "rust_axum",
        alias = "rs-axum",
        alias = "rs_axum",
        alias = "axum"
    )]
    RustAxum(ClientServerArgs),
    #[cfg(feature = "gen-rust-axum-adapters")]
    #[command(name = "rust-axum-adapters")]
    RustAxumAdapters(FilesArgs),
    #[cfg(feature = "gen-typescript")]
    #[command(name = "typescript", alias = "ts")]
    Typescript(ClientServerArgs),
    #[cfg(feature = "gen-typescript-rest")]
    #[command(
        name = "typescript-rest",
        alias = "typescript_rest",
        alias = "ts-rest",
        alias = "ts_rest"
    )]
    TypescriptRest(ClientServerArgs),
    #[cfg(feature = "gen-typescript-rest-adapters")]
    #[command(name = "typescript-rest-adapters")]
    TypescriptRestAdapters(FilesArgs),
    #[cfg(feature = "gen-go")]
    #[command(name = "go", alias = "golang")]
    Go(ClientServerArgs),
    #[cfg(feature = "gen-go-rest")]
    #[command(name = "go-rest", alias = "go_rest")]
    GoRest(ClientServerArgs),
    #[cfg(feature = "gen-openapi")]
    #[command(name = "openapi")]
    Openapi(OpenApiArgs),
    #[cfg(feature = "gen-openrpc")]
    #[command(name = "openrpc", alias = "open-rpc")]
    Openrpc(FilesArgs),
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Debug, Args)]
pub(super) struct FilesArgs {
    pub(super) files: Vec<PathBuf>,
}

/// Generates an OpenAPI JSON document for HTTP interfaces.
#[cfg(feature = "gen-openapi")]
#[derive(Debug, Args)]
pub(super) struct OpenApiArgs {
    /// OpenAPI document version. `auto` keeps the generator's choice (OpenAPI
    /// 3.1, upgraded to 3.2 when the contract has streams); `3.1` forces an
    /// OpenAPI 3.1 document, where streams are rendered best-effort.
    #[arg(long = "version", value_name = "VERSION", default_value = "auto")]
    pub(super) version: OpenApiVersion,
    pub(super) files: Vec<PathBuf>,
}

/// Accepted `--version` values for the `openapi` target.
#[cfg(feature = "gen-openapi")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(super) enum OpenApiVersion {
    /// Let the generator choose the document version.
    Auto,
    /// Force an OpenAPI 3.1 document.
    #[value(name = "3.1")]
    V31,
}

#[cfg(feature = "gen-openapi")]
impl OpenApiVersion {
    /// Generator property value, or `None` to keep the `auto` behaviour.
    fn property(self) -> Option<String> {
        match self {
            Self::Auto => None,
            Self::V31 => Some("3.1".to_string()),
        }
    }
}

#[cfg(any(
    feature = "gen-rust",
    feature = "gen-rust-jsonrpc",
    feature = "gen-rust-axum",
    feature = "gen-typescript",
    feature = "gen-typescript-rest",
    feature = "gen-go",
    feature = "gen-go-rest",
))]
#[derive(Debug, Args)]
pub(super) struct ClientServerArgs {
    #[arg(long = "client", default_value_t = false)]
    pub(super) client: bool,
    #[arg(long = "server", default_value_t = false)]
    pub(super) server: bool,
    #[arg(long = "mock", default_value_t = false)]
    pub(super) mock: bool,
    pub(super) files: Vec<PathBuf>,
}

#[derive(Debug)]
pub(super) struct SharedGenArgs {
    out_dir: String,
    dry_run: bool,
    api_version: Option<String>,
    api_versions: Option<String>,
}

impl ArgsGen {
    pub(super) fn into_driver_args(self) -> IdlcResult<ArgsGenerate> {
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
            #[cfg(feature = "gen-rust")]
            Self::Rust(args) => Ok(shared.into_client_server("rust", args)),
            #[cfg(feature = "gen-rust-jsonrpc")]
            Self::RustJsonRpc(args) => Ok(shared.into_client_server("rust-jsonrpc", args)),
            #[cfg(feature = "gen-rust-axum")]
            Self::RustAxum(args) => Ok(shared.into_client_server("rust-axum", args)),
            #[cfg(feature = "gen-rust-axum-adapters")]
            Self::RustAxumAdapters(args) => Ok(shared.into_plain("rust-axum-adapters", args.files)),
            #[cfg(feature = "gen-typescript")]
            Self::Typescript(args) => Ok(shared.into_client_server("typescript", args)),
            #[cfg(feature = "gen-typescript-rest")]
            Self::TypescriptRest(args) => Ok(shared.into_client_server("typescript-rest", args)),
            #[cfg(feature = "gen-typescript-rest-adapters")]
            Self::TypescriptRestAdapters(args) => {
                Ok(shared.into_plain("typescript-rest-adapters", args.files))
            }
            #[cfg(feature = "gen-go")]
            Self::Go(args) => Ok(shared.into_client_server("go", args)),
            #[cfg(feature = "gen-go-rest")]
            Self::GoRest(args) => Ok(shared.into_client_server("go-rest", args)),
            #[cfg(feature = "gen-openapi")]
            Self::Openapi(args) => {
                let mut generate = shared.into_plain("openapi", args.files);
                generate.openapi_version = args.version.property();
                Ok(generate)
            }
            #[cfg(feature = "gen-openrpc")]
            Self::Openrpc(args) => Ok(shared.into_plain("openrpc", args.files)),
            Self::External(values) => shared.into_external(values),
        }
    }

    pub(super) fn help_command(&self) -> clap::Command {
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
            #[cfg(feature = "gen-rust")]
            Self::Rust(_) => gen_cmd.find_subcommand_mut("rust"),
            #[cfg(feature = "gen-rust-jsonrpc")]
            Self::RustJsonRpc(_) => gen_cmd.find_subcommand_mut("rust-jsonrpc"),
            #[cfg(feature = "gen-rust-axum")]
            Self::RustAxum(_) => gen_cmd.find_subcommand_mut("rust-axum"),
            #[cfg(feature = "gen-rust-axum-adapters")]
            Self::RustAxumAdapters(_) => gen_cmd.find_subcommand_mut("rust-axum-adapters"),
            #[cfg(feature = "gen-typescript")]
            Self::Typescript(_) => gen_cmd.find_subcommand_mut("typescript"),
            #[cfg(feature = "gen-typescript-rest")]
            Self::TypescriptRest(_) => gen_cmd.find_subcommand_mut("typescript-rest"),
            #[cfg(feature = "gen-typescript-rest-adapters")]
            Self::TypescriptRestAdapters(_) => {
                gen_cmd.find_subcommand_mut("typescript-rest-adapters")
            }
            #[cfg(feature = "gen-go")]
            Self::Go(_) => gen_cmd.find_subcommand_mut("go"),
            #[cfg(feature = "gen-go-rest")]
            Self::GoRest(_) => gen_cmd.find_subcommand_mut("go-rest"),
            #[cfg(feature = "gen-openapi")]
            Self::Openapi(_) => gen_cmd.find_subcommand_mut("openapi"),
            #[cfg(feature = "gen-openrpc")]
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
            #[cfg(feature = "gen-rust")]
            Self::Rust(_) => "xidlc gen rust [OPTIONS] [FILES]...",
            #[cfg(feature = "gen-rust-jsonrpc")]
            Self::RustJsonRpc(_) => "xidlc gen rust-jsonrpc [OPTIONS] [FILES]...",
            #[cfg(feature = "gen-rust-axum")]
            Self::RustAxum(_) => "xidlc gen rust-axum [OPTIONS] [FILES]...",
            #[cfg(feature = "gen-rust-axum-adapters")]
            Self::RustAxumAdapters(_) => "xidlc gen rust-axum-adapters [FILES]...",
            #[cfg(feature = "gen-typescript")]
            Self::Typescript(_) => "xidlc gen typescript [OPTIONS] [FILES]...",
            #[cfg(feature = "gen-typescript-rest")]
            Self::TypescriptRest(_) => "xidlc gen typescript-rest [OPTIONS] [FILES]...",
            #[cfg(feature = "gen-typescript-rest-adapters")]
            Self::TypescriptRestAdapters(_) => "xidlc gen typescript-rest-adapters [FILES]...",
            #[cfg(feature = "gen-go")]
            Self::Go(_) => "xidlc gen go [OPTIONS] [FILES]...",
            #[cfg(feature = "gen-go-rest")]
            Self::GoRest(_) => "xidlc gen go-rest [OPTIONS] [FILES]...",
            #[cfg(feature = "gen-openapi")]
            Self::Openapi(_) => "xidlc gen openapi [OPTIONS] [FILES]...",
            #[cfg(feature = "gen-openrpc")]
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

    #[cfg(any(
        feature = "gen-rust",
        feature = "gen-rust-jsonrpc",
        feature = "gen-rust-axum",
        feature = "gen-typescript",
        feature = "gen-typescript-rest",
        feature = "gen-go",
        feature = "gen-go-rest",
    ))]
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

    fn into_external(self, values: Vec<OsString>) -> IdlcResult<ArgsGenerate> {
        let mut values = values.into_iter();
        let Some(lang) = values.next() else {
            return Err(IdlcError::fmt("missing generator language"));
        };
        let Some(lang) = lang.to_str() else {
            return Err(IdlcError::fmt("invalid utf-8 generator language"));
        };
        let files = values.map(PathBuf::from).collect::<Vec<_>>();
        Ok(self.into_plain(lang, files))
    }
}
