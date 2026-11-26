use crate::error::IdlcResult;
use crate::generate::typescript::definition::names::ts_ident;

use super::model::{MethodModel, ServerClassContext};
use super::renderer::TypescriptRestRenderer;

pub(super) struct ServerClass {
    context: ServerClassContext,
}

impl ServerClass {
    pub(super) fn new(
        interface_name: &str,
        module_path: &[String],
        methods: Vec<MethodModel>,
        fixed_response_headers: std::collections::BTreeMap<String, String>,
    ) -> crate::error::IdlcResult<Self> {
        Ok(Self {
            context: ServerClassContext {
                service_name: ts_ident(interface_name),
                methods: methods
                    .into_iter()
                    .map(|method| method.into_server_context(module_path))
                    .collect(),
                fixed_response_headers,
            },
        })
    }

    pub(super) fn render(&self, renderer: &TypescriptRestRenderer) -> IdlcResult<String> {
        renderer.render_template("server_class.ts.j2", &self.context)
    }
}
