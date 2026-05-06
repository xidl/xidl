use crate::error::{IdlcError, IdlcResult};
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;
use std::process::{Command, Stdio};

use super::GoRestRenderOutput;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/generate/go_rest/templates");

pub struct GoRestRenderer {
    env: Environment<'static>,
}

impl GoRestRenderer {
    pub fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        Ok(Self { env })
    }

    pub fn render_spec(&self, output: &GoRestRenderOutput) -> IdlcResult<String> {
        let rendered = self.render_template("spec.go.j2", output)?;
        format_go_source(&rendered)
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

fn format_go_source(source: &str) -> IdlcResult<String> {
    let Ok(mut child) = Command::new("gofmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    else {
        return Ok(source.to_string());
    };

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(source.as_bytes())
            .map_err(|err| IdlcError::template(format!("write gofmt stdin: {err}")))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| IdlcError::template(format!("wait for gofmt: {err}")))?;
    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|err| IdlcError::template(format!("decode gofmt output: {err}")))
    } else {
        Err(IdlcError::template(format!(
            "gofmt failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
