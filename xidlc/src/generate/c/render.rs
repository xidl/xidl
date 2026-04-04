use crate::{
    error::{IdlcError, IdlcResult},
    generate::utils::{clang_format_filter, format_timestamp_filter, to_case},
};
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/generate/c/templates");

pub struct CRenderer {
    env: Environment<'static>,
}

#[derive(Default)]
pub struct CRenderOutput {
    pub header: Vec<String>,
    pub source: Vec<String>,
    pub xcdr_header: Vec<String>,
    pub xcdr_source: Vec<String>,
}

impl CRenderOutput {
    pub fn push_header(mut self, value: String) -> Self {
        self.header.push(value);
        self
    }

    pub fn push_source(mut self, value: String) -> Self {
        self.source.push(value);
        self
    }

    pub fn push_xcdr_header(mut self, value: String) -> Self {
        self.xcdr_header.push(value);
        self
    }

    pub fn push_xcdr_source(mut self, value: String) -> Self {
        self.xcdr_source.push(value);
        self
    }

    pub fn extend(&mut self, other: CRenderOutput) {
        self.header.extend(other.header);
        self.source.extend(other.source);
        self.xcdr_header.extend(other.xcdr_header);
        self.xcdr_source.extend(other.xcdr_source);
    }
}

pub trait CRender {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput>;
}

impl CRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        env.add_filter("to_case", to_case);
        env.add_filter("clang-format", clang_format_filter);
        env.add_filter("fmt_timestamp", format_timestamp_filter);
        Ok(Self { env })
    }

    pub fn env(&mut self) -> &mut Environment<'static> {
        &mut self.env
    }

    pub fn render_template<T: Serialize>(&self, template: &str, ctx: &T) -> IdlcResult<String> {
        self.env
            .get_template(template)
            .map_err(|err| IdlcError::template(err.to_string()))?
            .render(ctx)
            .map_err(|err| IdlcError::template(err.to_string()))
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
