use crate::error::{IdlcError, IdlcResult};
use minijinja::{Environment, Error, ErrorKind};
use rust_embed::RustEmbed;
use serde::Serialize;

use super::GoHttpRenderOutput;

#[derive(RustEmbed)]
#[folder = "src/generate/go_http/templates"]
struct Templates;

pub struct GoHttpRenderer {
    env: Environment<'static>,
}

impl GoHttpRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        Ok(Self { env })
    }

    pub fn render_spec(&self, output: &GoHttpRenderOutput) -> IdlcResult<String> {
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
    let file = Templates::get(name).ok_or_else(|| {
        Error::new(
            ErrorKind::TemplateNotFound,
            format!("missing template {name}"),
        )
    })?;
    String::from_utf8(file.data.as_ref().to_vec()).map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("template {name} is not valid utf-8: {err}"),
        )
    })
}
