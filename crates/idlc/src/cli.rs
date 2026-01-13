use crate::driver;
use crate::error::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "idlc", about = "IDL translator", version)]
pub struct CliArgs {
    #[arg(long)]
    pub lang: String,
    #[arg(long = "out-dir")]
    pub out_dir: PathBuf,
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
}

pub fn run() -> Result<()> {
    let mut args = CliArgs::parse();
    args.lang = args.lang.to_ascii_lowercase();
    driver::run(args)
}
