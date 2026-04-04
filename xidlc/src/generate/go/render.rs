use crate::error::{IdlcError, IdlcResult};
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};

use super::GoRenderOutput;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/generate/go/templates");

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
