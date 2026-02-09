mod fmt;

mod generate;
pub use generate::ArgsGenerate;

use crate::driver;
use crate::error::IdlcResult;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "idlc", about = "IDL Compiler", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    generate: ArgsGenerate,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(alias = "format")]
    Fmt(fmt::ArgsFormat),
}

impl Cli {
    pub async fn run(self) -> IdlcResult<()> {
        match self.command {
            Some(Command::Fmt(args)) => args.execute(),
            None => driver::Driver::run(self.generate).await,
        }
    }
}
