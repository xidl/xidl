mod fmt;

use crate::driver;
use crate::driver::ArgsGenerate;
use crate::error::IdlcResult;
use clap::{CommandFactory, Parser, Subcommand};
use std::collections::HashMap;

#[derive(Debug, Parser)]
#[command(name = "idlc", about = "IDL Compiler", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(
        long = "skip-cdr-codec",
        default_value_t = false,
        help = "Skip generating CDR serialization and deserialization code"
    )]
    skip_cdr_codec: bool,
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
            None => {
                if self.generate.files.is_empty() {
                    Cli::command().print_help().unwrap();
                    return Ok(());
                }
                let mut props = HashMap::new();
                if self.skip_cdr_codec {
                    props.insert("enable_serialize".into(), false.into());
                    props.insert("enable_deserialize".into(), false.into());
                }
                driver::Driver::run_with_props(self.generate, props).await
            }
        }
    }
}
