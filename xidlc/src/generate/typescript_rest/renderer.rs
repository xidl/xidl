use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::typescript::TypescriptRenderer;
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;

static TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/src/generate/typescript_rest/templates");

/// Template renderer owning the TypeScript REST HTTP templates.
pub struct TypescriptRestRenderer {
    env: Environment<'static>,
    properties: HashMap<String, serde_json::Value>,
}

impl TypescriptRestRenderer {
    /// Creates a renderer backed by `typescript_rest/templates` with shared TS macros.
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        Ok(Self {
            env,
            properties: HashMap::new(),
        })
    }

    /// Renders `template` using `ctx`.
    pub fn render_template<T: Serialize>(&self, template: &str, ctx: &T) -> IdlcResult<String> {
        self.env
            .get_template(template)
            .map_err(|err| IdlcError::template(err.to_string()))?
            .render(ctx)
            .map_err(|err| IdlcError::template(err.to_string()))
    }

    /// Registers generator properties under the `opt` template global.
    pub fn extend(&mut self, props: &HashMap<String, serde_json::Value>) {
        self.env
            .add_global("opt", minijinja::Value::from_serialize(props));
        self.properties = props.clone();
    }

    /// Returns every imported unit prefix declared for this compilation unit.
    pub fn imported_units(&self) -> Vec<String> {
        crate::generate::utils::imported_units(&self.properties)
    }
}

fn load_template(name: &str) -> std::result::Result<String, Error> {
    if let Some(file) = TEMPLATES.get_file(name) {
        return file.contents_utf8().map(str::to_owned).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                format!("template {name} is not valid utf-8"),
            )
        });
    }
    TypescriptRenderer::load_shared_template(name)
}
