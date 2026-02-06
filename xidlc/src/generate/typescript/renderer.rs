use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use minijinja::{Environment, Error, ErrorKind};
use rust_embed::RustEmbed;
use serde::Serialize;

#[derive(RustEmbed)]
#[folder = "src/generate/typescript/templates"]
struct Templates;

pub struct TypescriptRenderer {
    env: Environment<'static>,
}

impl TypescriptRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        env.add_filter("ts", typescript_format_filter);
        env.add_filter("tsfmt", typescript_format_filter);
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
    let file = Templates::get(name).ok_or_else(|| {
        Error::new(
            ErrorKind::TemplateNotFound,
            format!("missing template {name}"),
        )
    })?;
    let data = file.data.as_ref();
    let content = String::from_utf8(data.to_vec()).map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("template {name} is not valid utf-8: {err}"),
        )
    })?;
    Ok(content)
}

fn typescript_format_filter(value: String) -> std::result::Result<String, Error> {
    crate::fmt::format_typescript_source(&value).map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("typescript format failed: {err}"),
        )
    })
}
