use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::names::ts_ident;

use super::model::{MethodModel, ServerClassContext};

pub(super) struct ServerClass {
    context: ServerClassContext,
}

impl ServerClass {
    pub(super) fn new(
        interface_name: &str,
        module_path: &[String],
        methods: Vec<MethodModel>,
    ) -> Self {
        Self {
            context: ServerClassContext {
                service_name: ts_ident(interface_name),
                methods: methods
                    .into_iter()
                    .map(|method| method.into_server_context(module_path))
                    .collect(),
            },
        }
    }

    pub(super) fn render(&self, renderer: &TypescriptRenderer) -> IdlcResult<String> {
        renderer.render_template("http/server_class.ts.j2", &self.context)
    }
}
