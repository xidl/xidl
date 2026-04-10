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

impl ArgsGenerate {
    pub fn generator_props(&self) -> HashMap<String, serde_json::Value> {
        HashMap::from([
            ("enable_client".into(), self.client.into()),
            ("enable_server".into(), self.server.into()),
        ])
    }
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
    extra_props: HashMap<String, serde_json::Value>,
}

impl Driver {
    pub async fn run(args: ArgsGenerate) -> IdlcResult<()> {
        Self::run_with_props(args, HashMap::new()).await
    }

    pub async fn run_with_props(
        args: ArgsGenerate,
        extra_props: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<()> {
        Self { args, extra_props }.execute().await
    }

    async fn execute(self) -> IdlcResult<()> {
        let output = match self.args.dry_run {
            true => out_file::OutputTarget::new_dummy(),
            false => out_file::OutputTarget::new_real(&self.args.out_dir)?,
        };

        let mut generator = generate::Generator::new(self.args.lang.clone());
        let mut props = self.args.generator_props();
        props.extend(self.extra_props);

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
