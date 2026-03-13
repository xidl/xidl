use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::{format_timestamp_filter, rust_format_filter};
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
    pub fn empty() -> Self {
        Self::default()
    }

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
        env.add_filter("rustfmt", rust_format_filter);
        env.add_filter("fmt_timestamp", format_timestamp_filter);
        env.add_global("opt", minijinja::Value::from_serialize(&attribute));
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

    pub fn render_source_template<T: Serialize>(
        &self,
        template: &str,
        ctx: &T,
    ) -> IdlcResult<RustRenderOutput> {
        self.render_template(template, ctx)
            .map(|rendered| RustRenderOutput::default().push_source(rendered))
    }

    pub fn render_module(&self, ident: &str, body: &str) -> IdlcResult<String> {
        self.render_template(
            "module.rs.j2",
            &serde_json::json!({
                "ident": crate::generate::rust::util::rust_ident(ident),
                "body": body,
            }),
        )
    }

    pub fn render_spec(&self, definitions: &[String]) -> IdlcResult<String> {
        self.render_template(
            "spec.rs.j2",
            &serde_json::json!({
                "definitions": definitions,
            }),
        )
    }

    pub fn enrich_ctx(&self, mut ctx: serde_json::Value, doc: &[String]) -> serde_json::Value {
        if let Some(obj) = ctx.as_object_mut() {
            obj.insert(
                "typeobject_path".to_string(),
                serde_json::json!(self.typeobject_path()),
            );
            obj.insert("doc".to_string(), serde_json::json!(doc));
        }
        ctx
    }

    pub fn with_ident(&self, mut ctx: serde_json::Value, ident: &str) -> serde_json::Value {
        if let Some(obj) = ctx.as_object_mut() {
            obj.insert(
                "ident".to_string(),
                serde_json::json!(crate::generate::rust::util::rust_ident(ident)),
            );
        }
        ctx
    }

    pub fn enrich_scoped_ctx(
        &self,
        ctx: serde_json::Value,
        doc: &[String],
        module_path: &[String],
    ) -> serde_json::Value {
        let mut ctx = self.enrich_ctx(ctx, doc);
        if let Some(obj) = ctx.as_object_mut() {
            obj.insert("module_path".to_string(), serde_json::json!(module_path));
        }
        ctx
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
