use crate::error::{IdlcError, IdlcResult};
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;
use xidl_parser::hir::ParserProperties;

use super::GoRenderOutput;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/generate/go/templates");

pub struct GoRenderer {
    env: Environment<'static>,
}

impl GoRenderer {
    pub fn new(properties: &ParserProperties) -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_keep_trailing_newline(true);
        env.set_loader(|name| load_template(name).map(Some));
        env.add_global("opt", minijinja::Value::from_serialize(properties));
        Ok(Self { env })
    }

    pub fn render_spec(&self, output: &GoRenderOutput) -> IdlcResult<String> {
        self.render_template("spec.go.j2", output)
    }

    pub fn render_template<S: Serialize>(&self, name: &str, ctx: &S) -> IdlcResult<String> {
        self.env
            .get_template(name)
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
