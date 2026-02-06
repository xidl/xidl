use crate::error::{IdlcError, IdlcResult};
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct GenerateArgs {
    #[arg(long = "lang", short = 'l')]
    lang: Option<String>,
    #[arg(long = "out-dir", short = 'o', default_value = ".")]
    out_dir: Option<String>,
    #[arg(long = "skip-client")]
    skip_client: bool,
    #[arg(long = "skip-server")]
    skip_server: bool,
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

impl GenerateArgs {
    pub fn into_cli_args(self) -> IdlcResult<CliArgs> {
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
