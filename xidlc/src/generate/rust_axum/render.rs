use crate::error::{IdlcError, IdlcResult};
use xidl_parser::http_hir::HttpHirDocument;
use crate::generate::utils::{format_timestamp_filter, rust_format_filter};
use convert_case::Casing;
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/generate/rust_axum/templates");

#[derive(Default)]
pub struct RustAxumRenderOutput {
    pub source: Vec<String>,
}

pub trait RustAxumRender {
    fn render(&self, renderer: &RustAxumRenderer) -> IdlcResult<RustAxumRenderOutput>;
}

pub struct RustAxumRenderer {
    env: Environment<'static>,
    http_hir: Option<HttpHirDocument>,
}

impl RustAxumRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        env.add_filter("rustfmt", rust_format_filter);
        env.add_filter("fmt_timestamp", format_timestamp_filter);
        env.add_filter("to_case", to_case);
        Ok(Self {
            env,
            http_hir: None,
        })
    }

    pub fn render_template<T: Serialize>(&self, template: &str, ctx: &T) -> IdlcResult<String> {
        self.env
            .get_template(template)
            .map_err(|err| IdlcError::template(err.to_string()))?
            .render(ctx)
            .map_err(|err| IdlcError::template(err.to_string()))
    }

    pub fn extend(
        &mut self,
        props: &std::collections::HashMap<String, serde_json::Value>,
        http_hir: HttpHirDocument,
    ) {
        self.http_hir = Some(http_hir);
        self.env
            .add_global("opt", minijinja::Value::from_serialize(props));
    }

    pub fn http_hir(&self) -> IdlcResult<HttpHirDocument> {
        self.http_hir
            .clone()
            .ok_or_else(|| IdlcError::rpc("missing http_hir in rust axum renderer".to_string()))
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

fn to_case(name: &str, case: &str) -> String {
    match case {
        "camel" => name.to_case(convert_case::Case::Camel),
        "snake_case" => name.to_case(convert_case::Case::Snake),
        _ => name.to_string(),
    }
}
