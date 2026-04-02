use crate::error::{IdlcError, IdlcResult};
use minijinja::{Environment, Error, ErrorKind};
use rust_embed::RustEmbed;

use super::GoRenderOutput;

#[derive(RustEmbed)]
#[folder = "src/generate/go/templates"]
struct Templates;

pub struct GoRenderer {
    env: Environment<'static>,
}

impl GoRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        Ok(Self { env })
    }

    pub fn render_spec(&self, output: &GoRenderOutput) -> IdlcResult<String> {
        self.env
            .get_template("spec.go.j2")
            .map_err(|err| IdlcError::template(err.to_string()))?
            .render(output)
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
