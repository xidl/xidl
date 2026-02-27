use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct ArgsGenerate {
    #[arg(long = "lang", short = 'l', default_value = "rust")]
    pub lang: String,
    #[arg(long = "out-dir", short = 'o', default_value = ".")]
    pub out_dir: String,
    #[arg(long = "client", default_value_t = false)]
    pub client: bool,
    #[arg(long = "server", default_value_t = false)]
    pub server: bool,
    #[arg(long = "ts", default_value_t = false)]
    pub ts: bool,
    #[arg(long = "dry-run", default_value_t = false)]
    pub dry_run: bool,
    pub files: Vec<PathBuf>,
}
