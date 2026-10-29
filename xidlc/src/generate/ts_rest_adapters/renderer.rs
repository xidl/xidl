//! Standalone renderer for the TypeScript REST adapter target.

use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use include_dir::{Dir, include_dir};
use minijinja::{Environment, Error, ErrorKind};
use serde::Serialize;

static TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/src/generate/ts_rest_adapters/templates");

/// Minijinja environment scoped to the adapter template directory so the
/// adapter never borrows the interface generator's template tree.
pub(crate) struct TsRestAdaptersRenderer {
    env: Environment<'static>,
}

impl TsRestAdaptersRenderer {
    pub(crate) fn new() -> IdlcResult<Self> {
        let mut env = Environment::new();
        env.set_loader(|name| load_template(name).map(Some));
        Ok(Self { env })
    }

    pub(crate) fn extend(&mut self, props: &HashMap<String, serde_json::Value>) {
        self.env
            .add_global("opt", minijinja::Value::from_serialize(props));
    }

    pub(crate) fn render_template<T: Serialize>(
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
