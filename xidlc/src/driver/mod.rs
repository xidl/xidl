mod generate;
pub(crate) mod lang;
pub use generate::Generator;

mod generate_session;
mod out_file;

use crate::driver::out_file::OutputTargetTrait;
use crate::error::IdlcResult;
use crate::library::{LibraryLoader, MOUNT_PROPERTY};
use crate::units::{IMPORT_PROPERTY, UNIT_PATH_PROPERTY, UnitGraph};
use std::collections::HashMap;
use std::path::PathBuf;
use xidl_parser::semantic::INTEGER_IMPORTS_PROPERTY;

/// Generator property selecting the OpenAPI document version.
///
/// `auto` (the default) keeps the projection's own choice, `3.1` forces an
/// OpenAPI 3.1 document. Set by the `openapi` target's `--version` option.
pub const OPENAPI_VERSION_PROPERTY: &str = "openapi_version";

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
    /// OpenAPI document version: `auto` (default) or `3.1`.
    pub openapi_version: Option<String>,
    pub files: Vec<PathBuf>,
}

impl ArgsGenerate {
    pub fn generator_props(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::from([
            ("enable_client".into(), self.client.into()),
            ("enable_server".into(), self.server.into()),
            ("mock".into(), self.mock.into()),
        ]);
        if let Some(version) = &self.openapi_version {
            props.insert(
                OPENAPI_VERSION_PROPERTY.to_string(),
                serde_json::Value::String(version.clone()),
            );
        }
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

        // Imports name other units, so the inputs are walked into a graph first
        // and every unit it reaches is generated exactly once.
        let graph = UnitGraph::build(resolved.files(), resolved.mounts())?;
        for unit in graph.units() {
            let mut unit_props = props.clone();
            unit_props.insert(IMPORT_PROPERTY.to_string(), graph.import_property(unit));
            unit_props.insert(
                INTEGER_IMPORTS_PROPERTY.to_owned(),
                serde_json::to_value(graph.integer_imports(unit))?,
            );
            unit_props.insert(
                UNIT_PATH_PROPERTY.to_string(),
                serde_json::Value::String(unit.path().to_string()),
            );
            let files =
                generator.generate_from_idl(unit.source(), &unit.generation_input(), unit_props)?;
            output.write_files(files)?;
        }

        Ok(())
    }
}
