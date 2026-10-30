use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::filter::is_upper_case;
use crate::generate::utils::rust_format_filter;
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/generate/rust/templates");

#[derive(Default)]
pub struct RustRenderOutput {
    pub source: Vec<String>,
}

#[derive(Serialize)]
struct RustModuleTemplate<'a> {
    ident: String,
    attrs: &'a [String],
    definitions: &'a [String],
}

#[derive(Serialize)]
struct RustSpecTemplate<'a> {
    definitions: &'a [String],
    imports: &'a [String],
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
    imports: Vec<String>,
    pub(super) runtime: Option<&'static str>,
}

impl RustRenderer {
    pub fn new(attribute: HashMap<String, serde_json::Value>) -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        env.add_function("is_upper", is_upper_case);
        env.add_filter("rustfmt", rust_format_filter);
        env.add_global("opt", minijinja::Value::from_serialize(&attribute));
        let imports = Self::import_prefixes_of(&attribute);
        Ok(Self {
            env,
            imports,
            runtime: None,
        })
    }

    /// Reference prefixes of the units this file imports, when it imports any.
    fn import_prefixes_of(attribute: &HashMap<String, serde_json::Value>) -> Vec<String> {
        attribute
            .get(crate::units::IMPORT_PROPERTY)
            .and_then(serde_json::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry.get("prefix"))
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The imported prefixes `definitions` actually reference.
    ///
    /// The rendered definitions already spell every reference, so a prefix
    /// appearing as `<prefix>::` is exactly a type this file reaches into
    /// another unit for.
    pub fn referenced_imports(&self, definitions: &[String]) -> Vec<String> {
        self.imports
            .iter()
            .filter(|prefix| {
                let marker = format!("{prefix}::");
                definitions
                    .iter()
                    .any(|definition| definition.contains(&marker))
            })
            .cloned()
            .collect()
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

    pub fn render_module(
        &self,
        ident: &str,
        definitions: &[String],
        attrs: &[String],
    ) -> IdlcResult<String> {
        self.render_template(
            "module.rs.j2",
            &RustModuleTemplate {
                ident: crate::generate::rust::util::rust_ident(ident),
                attrs,
                definitions,
            },
        )
    }

    pub fn render_spec(&self, definitions: &[String], imports: &[String]) -> IdlcResult<String> {
        self.render_template(
            "spec.rs.j2",
            &RustSpecTemplate {
                definitions,
                imports,
            },
        )
    }

    pub fn enrich_ctx(&self, mut ctx: serde_json::Value, doc: &[String]) -> serde_json::Value {
        if let Some(obj) = ctx.as_object_mut() {
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
