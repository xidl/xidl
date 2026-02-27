use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct ArgsGenerate {
    #[arg(long = "lang", short = 'l', default_value = "rust")]
    pub lang: String,
    #[arg(long = "out-dir", short = 'o', default_value = ".")]
    pub out_dir: String,
    #[arg(long = "client")]
    pub client: bool,
    #[arg(long = "server")]
    pub server: bool,
    #[arg(long = "ts")]
    pub ts: bool,
    pub files: Vec<PathBuf>,
}
