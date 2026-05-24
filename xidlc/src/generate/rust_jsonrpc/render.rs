use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::rust_format_filter;
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/generate/rust_jsonrpc/templates");

#[derive(Default)]
pub struct JsonRpcRenderOutput {
    pub source: Vec<String>,
}

pub trait JsonRpcRender {
    fn render(&self, renderer: &JsonRpcRenderer) -> IdlcResult<JsonRpcRenderOutput>;
}

pub struct JsonRpcRenderer {
    env: Environment<'static>,
}

impl JsonRpcRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        env.add_filter("rust", rust_format_filter);
        env.add_filter("rustfmt", rust_format_filter);
        Ok(Self { env })
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
