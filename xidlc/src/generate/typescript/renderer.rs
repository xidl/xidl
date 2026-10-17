use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/generate/typescript/templates");

// Force rebuild for template changes
pub struct TypescriptRenderer {
    env: Environment<'static>,
    properties: HashMap<String, serde_json::Value>,
}

impl TypescriptRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        Ok(Self {
            env,
            properties: HashMap::new(),
        })
    }

    pub fn render_template<T: Serialize>(&self, template: &str, ctx: &T) -> IdlcResult<String> {
        self.env
            .get_template(template)
            .map_err(|err| IdlcError::template(err.to_string()))?
            .render(ctx)
            .map_err(|err| IdlcError::template(err.to_string()))
    }

    pub fn extend(&mut self, props: &HashMap<String, serde_json::Value>) {
        self.env
            .add_global("opt", minijinja::Value::from_serialize(props));
        self.properties = props.clone();
    }

    /// Imported units `definitions` reference, by their prefixes.
    pub fn referenced_imports(&self, definitions: &[String]) -> Vec<String> {
        crate::generate::utils::referenced_imports(&self.properties, definitions, ".")
    }

    /// Every unit the file imports, by prefix.
    ///
    /// The model file is the module the rest of the file set imports from, so
    /// it re-exports each unit a reference may name, including one only a
    /// client or server file references.
    pub fn imported_units(&self) -> Vec<String> {
        crate::generate::utils::imported_units(&self.properties)
    }
}

fn load_template(name: &str) -> std::result::Result<String, Error> {
    let file = TEMPLATES.get_file(name).ok_or_else(|| {
        Error::new(
            ErrorKind::TemplateNotFound,
            format!("missing template {name}"),
        )
    })?;
    file.contents_utf8().map(str::to_owned).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("template {name} is not valid utf-8"),
        )
    })
}
