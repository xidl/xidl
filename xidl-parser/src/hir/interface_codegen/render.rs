use super::context::TemplateContext;
use crate::error::{ParseError, ParserResult};
use minijinja::{Environment, Error, ErrorKind};
use rust_embed::RustEmbed;

#[derive(RustEmbed, Clone)]
#[folder = "src/hir/templates"]
struct Templates;

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
    let file = Templates::get(name).ok_or_else(|| {
        Error::new(
            ErrorKind::TemplateNotFound,
            format!("missing template {name}"),
        )
    })?;
    let content = String::from_utf8(file.data.to_vec()).map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("template {name} is not valid utf-8: {err}"),
        )
    })?;
    Ok(content)
}
