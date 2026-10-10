mod exceptions;
mod helpers;
mod method;
mod render_types;
mod representations;
mod websocket_helpers;

use super::model::{ClientClassContext, MethodModel, TsHttpBlocks};
use super::server::ServerClass;
use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::names::ts_ident;
use render_types::{render_request_types, render_response_types};
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

pub(crate) fn render_interface(
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
    rest_hir: &RestHirDocument,
) -> IdlcResult<TsHttpBlocks> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(TsHttpBlocks::default());
    };
    let Some(http_interface) = rest_hir.find_interface(module_path, &def.header.ident) else {
        return Ok(TsHttpBlocks::default());
    };
    let methods = http_interface
        .operations
        .iter()
        .map(|op| {
            MethodModel::for_operation(
                def.header.ident.as_str(),
                module_path,
                op,
                &rest_hir.document.exceptions,
            )
        })
        .collect::<IdlcResult<Vec<_>>>()?;
    let mut out = TsHttpBlocks::default();
    for method in &methods {
        render_request_types(&mut out, method, module_path, renderer)?;
        render_response_types(&mut out, method, module_path, renderer)?;
    }
    out.client.push(
        renderer.render_template(
            "http/client_class.ts.j2",
            &ClientClassContext {
                client_name: ts_ident(&def.header.ident), // Template adds 'Client'
                methods: methods
                    .iter()
                    .cloned()
                    .map(MethodModel::into_client_context)
                    .collect(),
            },
        )?,
    );
    let (ws_client_helpers, ws_server_helpers) =
        websocket_helpers::render_websocket_helpers(&methods);
    out.client.extend(ws_client_helpers);
    out.server.extend(ws_server_helpers);
    out.server
        .push(ServerClass::new(&def.header.ident, module_path, methods).render(renderer)?);

    Ok(out)
}
