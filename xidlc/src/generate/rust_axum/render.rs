use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::format_timestamp_filter;
use minijinja::{Environment, Error, ErrorKind};
use rust_embed::RustEmbed;
use serde::Serialize;

#[derive(RustEmbed)]
#[folder = "src/generate/rust_axum/templates"]
struct Templates;

#[derive(Default)]
pub struct RustAxumRenderOutput {
    pub source: Vec<String>,
}

pub trait RustAxumRender {
    fn render(&self, renderer: &RustAxumRenderer) -> IdlcResult<RustAxumRenderOutput>;
}

pub struct RustAxumRenderer {
    env: Environment<'static>,
}

impl RustAxumRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        env.add_filter("rust", rust_format_filter);
        env.add_filter("rustfmt", rust_format_filter);
        env.add_filter("fmt_timestamp", format_timestamp_filter);
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

fn rust_format_filter(value: String) -> std::result::Result<String, Error> {
    crate::fmt::format_rust_source(&value).map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("rust format failed: {err}"),
        )
    })
}
