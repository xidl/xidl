mod fmt;

use crate::driver;
use crate::driver::ArgsGenerate;
use crate::error::{IdlcError, IdlcResult};
use clap::{Args, CommandFactory, Parser, Subcommand};
use std::collections::HashMap;
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
    Gen(ArgsGen),
    #[command(alias = "format")]
    Fmt(fmt::ArgsFormat),
}

#[derive(Debug, Args)]
struct ArgsGen {
    #[arg(long = "out-dir", short = 'o', default_value = ".")]
    out_dir: String,
    #[arg(long = "dry-run", default_value_t = false)]
    dry_run: bool,
    #[arg(
        long = "skip-cdr-codec",
        default_value_t = false,
        help = "Skip generating CDR serialization and deserialization code"
    )]
    skip_cdr_codec: bool,
    #[command(subcommand)]
    lang: GenLang,
}

#[derive(Debug, Subcommand)]
enum GenLang {
    #[command(name = "hir", hide = true)]
    Hir(FilesArgs),
    #[command(name = "http-hir", alias = "http_hir", hide = true)]
    HttpHir(FilesArgs),
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
        name = "typescript-http",
        alias = "typescript_http",
        alias = "ts-http",
        alias = "ts_http"
    )]
    TypescriptHttp(ClientOnlyArgs),
    #[command(name = "go", alias = "golang")]
    Go(ClientServerArgs),
    #[command(name = "go-http", alias = "go_http")]
    GoHttp(ClientServerArgs),
    #[command(name = "python", alias = "py")]
    Python(ClientServerArgs),
    #[command(
        name = "python-http",
        alias = "python_http",
        alias = "py-http",
        alias = "py_http"
    )]
    PythonHttp(ClientServerArgs),
    #[command(name = "openapi")]
    Openapi(FilesArgs),
    #[command(name = "openrpc", alias = "open-rpc")]
    Openrpc(FilesArgs),
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Debug, Args)]
struct FilesArgs {
    files: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct ClientServerArgs {
    #[arg(long = "client", default_value_t = false)]
    client: bool,
    #[arg(long = "server", default_value_t = true)]
    server: bool,
    files: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct ClientOnlyArgs {
    #[arg(long = "client", default_value_t = true)]
    client: bool,
    files: Vec<PathBuf>,
}

#[derive(Debug)]
struct SharedGenArgs {
    out_dir: String,
    dry_run: bool,
}

impl ArgsGen {
    fn into_driver_args(self) -> IdlcResult<(ArgsGenerate, HashMap<String, serde_json::Value>)> {
        let mut extra_props = HashMap::new();
        if self.skip_cdr_codec {
            extra_props.insert("enable_serialize".into(), false.into());
            extra_props.insert("enable_deserialize".into(), false.into());
        }
        let shared = SharedGenArgs {
            out_dir: self.out_dir,
            dry_run: self.dry_run,
        };
        let args = self.lang.into_driver_args(shared)?;
        Ok((args, extra_props))
    }
}

impl GenLang {
    fn into_driver_args(self, shared: SharedGenArgs) -> IdlcResult<ArgsGenerate> {
        match self {
            Self::Hir(args) => Ok(shared.into_plain("hir", args.files)),
            Self::HttpHir(args) => Ok(shared.into_plain("http-hir", args.files)),
            Self::JsonRpcHir(args) => Ok(shared.into_plain("jsonrpc-hir", args.files)),
            Self::TypedAst(args) => Ok(shared.into_plain("typed-ast", args.files)),
            Self::Rust(args) => Ok(shared.into_client_server("rust", args)),
            Self::RustJsonRpc(args) => Ok(shared.into_client_server("rust-jsonrpc", args)),
            Self::RustAxum(args) => Ok(shared.into_client_server("rust-axum", args)),
            Self::Typescript(args) => Ok(shared.into_client_server("typescript", args)),
            Self::TypescriptHttp(args) => Ok(shared.into_client_only("typescript-http", args)),
            Self::Go(args) => Ok(shared.into_client_server("go", args)),
            Self::GoHttp(args) => Ok(shared.into_client_server("go-http", args)),
            Self::Python(args) => Ok(shared.into_client_server("python", args)),
            Self::PythonHttp(args) => Ok(shared.into_client_server("python-http", args)),
            Self::Openapi(args) => Ok(shared.into_plain("openapi", args.files)),
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
            Self::HttpHir(_) => gen_cmd.find_subcommand_mut("http-hir"),
            Self::JsonRpcHir(_) => gen_cmd.find_subcommand_mut("jsonrpc-hir"),
            Self::TypedAst(_) => gen_cmd.find_subcommand_mut("typed-ast"),
            Self::Rust(_) => gen_cmd.find_subcommand_mut("rust"),
            Self::RustJsonRpc(_) => gen_cmd.find_subcommand_mut("rust-jsonrpc"),
            Self::RustAxum(_) => gen_cmd.find_subcommand_mut("rust-axum"),
            Self::Typescript(_) => gen_cmd.find_subcommand_mut("typescript"),
            Self::TypescriptHttp(_) => gen_cmd.find_subcommand_mut("typescript-http"),
            Self::Go(_) => gen_cmd.find_subcommand_mut("go"),
            Self::GoHttp(_) => gen_cmd.find_subcommand_mut("go-http"),
            Self::Python(_) => gen_cmd.find_subcommand_mut("python"),
            Self::PythonHttp(_) => gen_cmd.find_subcommand_mut("python-http"),
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
            Self::HttpHir(_) => "xidlc gen http-hir [FILES]...",
            Self::JsonRpcHir(_) => "xidlc gen jsonrpc-hir [FILES]...",
            Self::TypedAst(_) => "xidlc gen typed-ast [FILES]...",
            Self::Rust(_) => "xidlc gen rust [OPTIONS] [FILES]...",
            Self::RustJsonRpc(_) => "xidlc gen rust-jsonrpc [OPTIONS] [FILES]...",
            Self::RustAxum(_) => "xidlc gen rust-axum [OPTIONS] [FILES]...",
            Self::Typescript(_) => "xidlc gen typescript [OPTIONS] [FILES]...",
            Self::TypescriptHttp(_) => "xidlc gen typescript-http [OPTIONS] [FILES]...",
            Self::Go(_) => "xidlc gen go [OPTIONS] [FILES]...",
            Self::GoHttp(_) => "xidlc gen go-http [OPTIONS] [FILES]...",
            Self::Python(_) => "xidlc gen python [OPTIONS] [FILES]...",
            Self::PythonHttp(_) => "xidlc gen python-http [OPTIONS] [FILES]...",
            Self::Openapi(_) => "xidlc gen openapi [FILES]...",
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
            files,
        }
    }

    fn into_client_server(self, lang: impl Into<String>, args: ClientServerArgs) -> ArgsGenerate {
        ArgsGenerate {
            lang: lang.into(),
            out_dir: self.out_dir,
            client: args.client,
            server: args.server,
            dry_run: self.dry_run,
            files: args.files,
        }
    }

    fn into_client_only(self, lang: impl Into<String>, args: ClientOnlyArgs) -> ArgsGenerate {
        ArgsGenerate {
            lang: lang.into(),
            out_dir: self.out_dir,
            client: args.client,
            server: false,
            dry_run: self.dry_run,
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

impl Cli {
    pub async fn run(self) -> IdlcResult<()> {
        match self.command {
            Command::Gen(args) => {
                let help_command = args.lang.help_command();
                let (args, extra_props) = args.into_driver_args()?;
                if args.files.is_empty() {
                    help_command.clone().print_help().unwrap();
                    println!();
                    return Ok(());
                }
                driver::Driver::run_with_props(args, extra_props).await
            }
            Command::Fmt(args) => args.execute(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gen_subcommand_with_lang_enum_and_files() {
        let cli = Cli::try_parse_from(["xidlc", "gen", "rust", "demo.idl"]).expect("parse cli");
        match cli.command {
            Command::Gen(args) => match args.lang {
                GenLang::Rust(lang) => {
                    assert_eq!(lang.files, vec![PathBuf::from("demo.idl")]);
                    assert!(!lang.client);
                    assert!(lang.server);
                }
                _ => panic!("expected rust lang"),
            },
            Command::Fmt(_) => panic!("expected gen command"),
        }
    }
}
