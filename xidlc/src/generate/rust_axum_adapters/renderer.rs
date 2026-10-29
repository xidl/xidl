//! Standalone renderer for the Rust Axum adapter target.

use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::rust_format_filter;
use convert_case::Casing;
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};

static TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/src/generate/rust_axum_adapters/templates");

/// Minijinja environment scoped to the adapter template directory so the
/// adapter never borrows the interface generator's template tree.
pub(crate) struct RustAxumAdaptersRenderer {
    env: Environment<'static>,
}

impl RustAxumAdaptersRenderer {
    pub(crate) fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        env.add_filter("rustfmt", rust_format_filter);
        env.add_filter("to_case", to_case);
        Ok(Self { env })
    }

    pub(crate) fn render_template<T: serde::Serialize>(
        &self,
        template: &str,
        ctx: &T,
    ) -> IdlcResult<String> {
        self.env
            .get_template(template)
            .map_err(|err| IdlcError::template(err.to_string()))?
            .render(ctx)
            .map_err(|err| IdlcError::template(err.to_string()))
    }
}

fn to_case(name: &str, case: &str) -> String {
    match case {
        "camel" => name.to_case(convert_case::Case::Camel),
        "snake_case" => name.to_case(convert_case::Case::Snake),
        _ => name.to_string(),
    }
}

fn load_template(name: &str) -> std::result::Result<String, Error> {
    let file = TEMPLATES.get_file(name).ok_or_else(|| {
        Error::new(
            ErrorKind::TemplateNotFound,
            format!("missing adapter template {name}"),
        )
    })?;
    file.contents_utf8()
        .map(str::to_string)
        .ok_or_else(|| Error::new(ErrorKind::InvalidOperation, "template is not utf-8"))
}
