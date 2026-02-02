use crate::driver;
use crate::error::{IdlcError, IdlcResult};
use clap::{Args, Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "idlc", about = "IDL Compiler", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    generate: GenerateArgs,
}

#[derive(Debug, Args)]
struct GenerateArgs {
    #[arg(long = "lang", short = 'l')]
    lang: Option<String>,
    #[arg(long = "out-dir", short = 'o')]
    out_dir: Option<String>,
    inputs: Vec<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(alias = "format")]
    Fmt(FormatArgs),
}

#[derive(Debug, Args)]
struct FormatArgs {
    #[arg(long = "lang", short = 'l', value_enum, default_value_t = FormatLang::Idl)]
    lang: FormatLang,
    #[arg(long = "out-dir", short = 'o', default_value = "-")]
    out_dir: String,
    #[arg(long)]
    write: bool,
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct CliArgs {
    pub lang: String,
    pub out_dir: String,
    pub inputs: Vec<PathBuf>,
}

pub fn run() -> IdlcResult<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Fmt(args)) => run_format(args),
        None => {
            if cli.generate.inputs.is_empty() {
                return Err(IdlcError::fmt("missing input files".to_string()));
            }
            let mut args = CliArgs {
                lang: cli
                    .generate
                    .lang
                    .ok_or_else(|| IdlcError::fmt("missing --lang".to_string()))?,
                out_dir: cli
                    .generate
                    .out_dir
                    .ok_or_else(|| IdlcError::fmt("missing --out-dir".to_string()))?,
                inputs: cli.generate.inputs,
            };
            args.lang = args.lang.to_ascii_lowercase();
            driver::run(args)
        }
    }
}

fn run_format(args: FormatArgs) -> IdlcResult<()> {
    if args.inputs.len() > 1 && args.out_dir == "-" && !args.write {
        return Err(IdlcError::fmt(
            "multiple inputs require --write or --out-dir".to_string(),
        ));
    }

    for (idx, input) in args.inputs.iter().enumerate() {
        let source = std::fs::read_to_string(input)?;
        let formatted = match args.lang {
            FormatLang::Idl => crate::fmt::format_idl_source(&source)?,
            FormatLang::Rust => crate::fmt::format_rust_source(&source)?,
            FormatLang::C => crate::fmt::format_c_source(&source)?,
            FormatLang::Cpp => crate::fmt::format_c_source(&source)?,
        };
        if args.write {
            std::fs::write(input, formatted)?;
        } else if args.out_dir == "-" {
            if idx > 0 {
                println!();
            }
            print!("{formatted}");
        } else {
            let out_path = format_output_path(&args.out_dir, input);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(out_path, formatted)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum FormatLang {
    Idl,
    #[value(aliases = ["rs"])]
    Rust,
    C,
    #[value(aliases = ["cpp", "c++"])]
    Cpp,
}

fn format_output_path(out_dir: &str, input: &Path) -> PathBuf {
    let file_name = input
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("output.idl"));
    Path::new(out_dir).join(file_name)
}
