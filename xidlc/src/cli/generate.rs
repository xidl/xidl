use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct ArgsGenerate {
    #[arg(long = "lang", short = 'l', default_value = "rust")]
    pub lang: String,
    #[arg(long = "out-dir", short = 'o', default_value = ".")]
    pub out_dir: String,
    #[arg(long = "skip-client")]
    pub skip_client: bool,
    #[arg(long = "skip-server")]
    pub skip_server: bool,
    pub files: Vec<PathBuf>,
}
