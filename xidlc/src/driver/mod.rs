mod generate;
pub(crate) mod lang;
pub use generate::Generator;

mod generate_session;
mod out_file;

use crate::driver::out_file::OutputTargetTrait;
use crate::error::IdlcResult;
use crate::library::{LibraryLoader, MOUNT_PROPERTY};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ArgsGenerate {
    pub lang: String,
    pub out_dir: String,
    pub client: bool,
    pub server: bool,
    pub dry_run: bool,
    pub mock: bool,
    /// One API version of a versioned contract, by member ident or value.
    pub api_version: Option<String>,
    /// Several API versions, comma separated, or `*` for every version.
    pub api_versions: Option<String>,
    /// Serve older versions from the head one by generating version adapters.
    pub version_adapters: bool,
    pub files: Vec<PathBuf>,
}

impl ArgsGenerate {
    pub fn generator_props(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::from([
            ("enable_client".into(), self.client.into()),
            ("enable_server".into(), self.server.into()),
            ("mock".into(), self.mock.into()),
        ]);
        if let Some(version) = &self.api_version {
            props.insert(
                xidl_parser::versioning::API_VERSION_PROPERTY.to_string(),
                serde_json::Value::String(version.clone()),
            );
        }
        if let Some(versions) = &self.api_versions {
            props.insert(
                xidl_parser::versioning::API_VERSIONS_PROPERTY.to_string(),
                serde_json::Value::String(versions.clone()),
            );
        }
        if self.version_adapters {
            props.insert(
                crate::generate::adapter::entry::ENABLE_VERSION_ADAPTERS_PROPERTY.to_string(),
                true.into(),
            );
        }
        props
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
    pub fn run(args: ArgsGenerate) -> IdlcResult<()> {
        Self::run_with_props(args, HashMap::new())
    }

    pub fn run_with_props(
        args: ArgsGenerate,
        extra_props: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<()> {
        Self { args, extra_props }.execute()
    }

    fn execute(self) -> IdlcResult<()> {
        let output = match self.args.dry_run {
            true => out_file::OutputTarget::new_dummy(),
            false => out_file::OutputTarget::new_real(&self.args.out_dir)?,
        };

        // Library arguments are expanded before generation, so a directory
        // holding `xidl.toml` behaves like the list of files it declares.
        let resolved = LibraryLoader::new().resolve(&self.args.files)?;
        resolved.check_compiler()?;

        let mut generator = generate::Generator::new(self.args.lang.clone());
        let mut props = self.args.generator_props();
        if !resolved.mounts().is_empty() {
            props.insert(MOUNT_PROPERTY.to_string(), resolved.mount_property());
        }
        props.extend(self.extra_props);

        for input in resolved.files() {
            let source = fs::read_to_string(input)?;
            let files = generator.generate_from_idl(&source, input, props.clone())?;
            output.write_files(files)?;
        }

        Ok(())
    }
}
