use super::context::TemplateContext;
use crate::error::{ParseError, ParserResult};
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/hir/templates");

pub fn render_template(name: &str, ctx: &TemplateContext) -> ParserResult<String> {
    let mut env = Environment::new();
    let source = load_template(name).map_err(|err| ParseError::TreeSitterError(err.to_string()))?;
    env.add_template(name, &source)
        .map_err(|err| ParseError::TreeSitterError(err.to_string()))?;
    let template = env
        .get_template(name)
        .map_err(|err| ParseError::TreeSitterError(err.to_string()))?;
    template
        .render(ctx)
        .map_err(|err| ParseError::TreeSitterError(err.to_string()))
}

fn load_template(name: &str) -> std::result::Result<String, Error> {
    let file = TEMPLATES.get_file(name).ok_or_else(|| {
        Error::new(
            ErrorKind::TemplateNotFound,
            format!("missing template {name}"),
        )
    })?;
    let content = file.contents_utf8().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("template {name} is not valid utf-8"),
        )
    })?;
    Ok(content.to_string())
}
