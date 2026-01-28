use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use minijinja::{Environment, Error, ErrorKind};
use rust_embed::RustEmbed;
use serde::Serialize;

#[derive(RustEmbed)]
#[folder = "src/generate/rust/templates"]
struct Templates;

#[derive(Default)]
pub struct RustRenderOutput {
    pub source: Vec<String>,
}

impl RustRenderOutput {
    pub fn push_source(mut self, value: String) -> Self {
        self.source.push(value);
        self
    }

    pub fn extend(&mut self, other: RustRenderOutput) {
        self.source.extend(other.source);
    }
}

pub trait RustRender {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput>;
}

pub struct RustRenderer {
    env: Environment<'static>,
    typeobject_path: String,
}

impl RustRenderer {
    pub fn new(
        typeobject_path: String,
        attribute: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        env.add_function("md5_14", |value: String| md5_prefix(value.as_bytes(), 14));
        env.add_function("md5_4", |value: String| md5_prefix(value.as_bytes(), 4));
        env.add_function("name_hash", |value: String| md5_prefix(value.as_bytes(), 4));
        for (k, v) in attribute {
            env.add_global(k, minijinja::Value::from_serialize(v));
        }
        Ok(Self {
            env,
            typeobject_path,
        })
    }

    pub fn render_template<T: Serialize>(&self, template: &str, ctx: &T) -> IdlcResult<String> {
        self.env
            .get_template(template)
            .map_err(|err| IdlcError::template(err.to_string()))?
            .render(ctx)
            .map_err(|err| IdlcError::template(err.to_string()))
    }

    pub fn typeobject_path(&self) -> &str {
        self.typeobject_path.as_str()
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

fn md5_prefix(input: &[u8], len: usize) -> Vec<u8> {
    let digest = md5::compute(input);
    let end = len.min(digest.0.len());
    digest.0[..end].to_vec()
}
