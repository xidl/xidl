#[cfg(test)]
mod tests;

mod generate;
pub use generate::Generator;

mod generate_session;
mod out_file;

use crate::cli::ArgsGenerate;
use crate::driver::out_file::OutputTargetTrait;
use crate::error::IdlcResult;
use std::collections::HashMap;
use std::fs;

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
        props.insert("enable_ts".into(), self.args.ts.into());

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

#[cfg(test)]
pub async fn generate_from_idl(
    source: &str,
    path: &std::path::Path,
    lang: &str,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<File>> {
    let mut generator = generate::Generator::new(lang.into());
    generator.generate_from_idl(source, path, props).await
}
