use crate::driver;
use crate::error::IdlcResult;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "idlc", about = "IDL Compiler", version)]
pub struct CliArgs {
    #[arg(long = "lang", short = 'l')]
    pub lang: String,
    #[arg(long = "out-dir", short = 'o')]
    pub out_dir: String,
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
}

pub fn run() -> IdlcResult<()> {
    let mut args = CliArgs::parse();
    args.lang = args.lang.to_ascii_lowercase();
    driver::run(args)
}
