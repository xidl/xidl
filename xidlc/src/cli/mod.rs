mod fmt;
mod highlight;

mod generate;
pub use generate::CliArgs;

use crate::driver;
use crate::error::IdlcResult;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "idlc", about = "IDL Compiler", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    generate: generate::GenerateArgs,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(alias = "format")]
    Fmt(fmt::FormatArgs),
    #[command(hide = true)]
    Highlight(highlight::HighlightArgs),
}

impl Cli {
    pub async fn run(self) -> IdlcResult<()> {
        match self.command {
            Some(Command::Fmt(args)) => args.execute(),
            Some(Command::Highlight(args)) => args.execute(),
            None => {
                let args = self.generate.into_cli_args()?;
                driver::Driver::run(args).await
            }
        }
    }
}
