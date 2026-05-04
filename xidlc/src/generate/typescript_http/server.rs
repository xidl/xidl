use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::names::ts_ident;

use super::model::{MethodModel, ServerClassContext};

pub(super) fn render_server_block(
    interface_name: &str,
    module_path: &[String],
    methods: Vec<MethodModel>,
    renderer: &TypescriptRenderer,
) -> IdlcResult<String> {
    renderer.render_template(
        "http/server_class.ts.j2",
        &ServerClassContext {
            service_name: ts_ident(interface_name),
            handler_name: format!("create{}Handler", ts_ident(interface_name)),
            methods: methods
                .into_iter()
                .map(|method| method.into_server_context(module_path))
                .collect(),
        },
    )
}
