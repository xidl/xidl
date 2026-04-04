mod generate;
mod lang;
pub use generate::Generator;

mod generate_session;
mod out_file;

use crate::driver::out_file::OutputTargetTrait;
use crate::error::IdlcResult;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(clap::Args))]
pub struct ArgsGenerate {
    #[cfg_attr(
        feature = "cli",
        arg(long = "lang", short = 'l', default_value = "rust")
    )]
    pub lang: String,
    #[cfg_attr(
        feature = "cli",
        arg(long = "out-dir", short = 'o', default_value = ".")
    )]
    pub out_dir: String,
    #[cfg_attr(feature = "cli", arg(long = "client", default_value_t = false))]
    pub client: bool,
    #[cfg_attr(feature = "cli", arg(long = "server", default_value_t = true))]
    pub server: bool,
    #[cfg_attr(feature = "cli", arg(long = "dry-run", default_value_t = false))]
    pub dry_run: bool,
    pub files: Vec<PathBuf>,
}

pub struct File {
    path: String,
    content: String,
}

impl File {
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn content(&self) -> &str {
        self.content.as_str()
    }
}

pub struct Driver {
    args: ArgsGenerate,
}

impl Driver {
    pub async fn run(args: ArgsGenerate) -> IdlcResult<()> {
        Self { args }.execute().await
    }

    async fn execute(self) -> IdlcResult<()> {
        let output = match self.args.dry_run {
            true => out_file::OutputTarget::new_dummy(),
            false => out_file::OutputTarget::new_real(&self.args.out_dir)?,
        };

        let mut generator = generate::Generator::new(self.args.lang.clone());
        let mut props = HashMap::new();
        props.insert("enable_client".into(), self.args.client.into());
        props.insert("enable_server".into(), self.args.server.into());

        for input in self.args.files {
            let source = fs::read_to_string(&input)?;
            let files = generator
                .generate_from_idl(&source, &input, props.clone())
                .await?;
            output.write_files(files)?;
        }

        Ok(())
    }
}
