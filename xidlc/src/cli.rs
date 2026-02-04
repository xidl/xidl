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
    #[arg(long = "skip-client")]
    skip_client: bool,
    #[arg(long = "skip-server")]
    skip_server: bool,
    inputs: Vec<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(alias = "format")]
    Fmt(FormatArgs),
    #[command(hide = true)]
    Highlight(HighlightArgs),
}

#[derive(Debug, Args)]
struct FormatArgs {
    #[arg(long = "lang", short = 'l', value_enum, default_value_t = FormatLang::Idl)]
    lang: FormatLang,
    #[arg(long = "out-dir", short = 'o', default_value = "-")]
    out_dir: String,
    /// Format inplace.
    #[arg(long, short = 'i')]
    inplace: bool,
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct HighlightArgs {
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
    pub skip_client: bool,
    pub skip_server: bool,
}

impl Cli {
    pub fn run(self) -> IdlcResult<()> {
        match self.command {
            Some(Command::Fmt(args)) => args.execute(),
            Some(Command::Highlight(args)) => args.execute(),
            None => {
                let args = self.generate.into_cli_args()?;
                driver::Driver::run(args)
            }
        }
    }
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

impl GenerateArgs {
    fn into_cli_args(self) -> IdlcResult<CliArgs> {
        if self.inputs.is_empty() {
            return Err(IdlcError::fmt("missing input files".to_string()));
        }
        let lang = self
            .lang
            .ok_or_else(|| IdlcError::fmt("missing --lang".to_string()))?
            .to_ascii_lowercase();
        let out_dir = self
            .out_dir
            .ok_or_else(|| IdlcError::fmt("missing --out-dir".to_string()))?;
        Ok(CliArgs {
            lang,
            out_dir,
            inputs: self.inputs,
            skip_client: self.skip_client,
            skip_server: self.skip_server,
        })
    }
}

impl FormatArgs {
    fn execute(self) -> IdlcResult<()> {
        if self.inputs.len() > 1 && self.out_dir == "-" && !self.inplace {
            return Err(IdlcError::fmt(
                "multiple inputs require --write or --out-dir".to_string(),
            ));
        }

        for (idx, input) in self.inputs.iter().enumerate() {
            let source = std::fs::read_to_string(input)?;
            let formatted = match self.lang {
                FormatLang::Idl => crate::fmt::format_idl_source(&source)?,
                FormatLang::Rust => crate::fmt::format_rust_source(&source)?,
                FormatLang::C => crate::fmt::format_c_source(&source)?,
                FormatLang::Cpp => crate::fmt::format_c_source(&source)?,
            };
            if self.inplace {
                std::fs::write(input, formatted)?;
            } else if self.out_dir == "-" {
                if idx > 0 {
                    println!();
                }
                print!("{formatted}");
            } else {
                let out_path = self.format_output_path(input);
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(out_path, formatted)?;
            }
        }

        Ok(())
    }

    fn format_output_path(&self, input: &Path) -> PathBuf {
        let file_name = input
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("output.idl"));
        Path::new(&self.out_dir).join(file_name)
    }
}

impl HighlightArgs {
    fn execute(self) -> IdlcResult<()> {
        if self.inputs.len() > 1 && self.out_dir == "-" && !self.write {
            return Err(IdlcError::fmt(
                "multiple inputs require --write or --out-dir".to_string(),
            ));
        }

        for (idx, input) in self.inputs.iter().enumerate() {
            let source = std::fs::read_to_string(input)?;
            let highlighted =
                crate::highlight::highlight_idl(&source, input.to_string_lossy().as_ref())?;
            if self.write {
                std::fs::write(input, highlighted.text)?;
            } else if self.out_dir == "-" {
                if idx > 0 {
                    println!();
                }
                print!("{}", highlighted.text);
            } else {
                let out_path = self.highlight_output_path(input);
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(out_path, highlighted.text)?;
            }
        }

        Ok(())
    }

    fn highlight_output_path(&self, input: &Path) -> PathBuf {
        let file_name = input
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("output.idl"));
        Path::new(&self.out_dir).join(file_name)
    }
}
