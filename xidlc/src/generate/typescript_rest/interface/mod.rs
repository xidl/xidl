use super::model::{ClientClassContext, MethodModel, TsHttpBlocks};
use super::server::ServerClass;
use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::names::ts_ident;
use method::OperationModelInput;
use render_types::{render_request_types, render_response_types};
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

mod exceptions;
mod helpers;
mod method;
mod render_types;
mod representations;
mod websocket_helpers;

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
    let unions = switched_union_idents(&rest_hir.spec);
    let methods = http_interface
        .operations
        .iter()
        .map(|op| {
            MethodModel::for_operation(OperationModelInput {
                interface_name: def.header.ident.as_str(),
                module_path,
                op,
                exceptions: &rest_hir.document.exceptions,
                union_idents: &unions,
            })
        })
        .collect::<IdlcResult<Vec<_>>>()?;
    let mut out = TsHttpBlocks::default();
    for method in &methods {
        render_request_types(&mut out, method, module_path, renderer)?;
        render_response_types(&mut out, method, module_path, renderer)?;
    }
    // The version this client pins: the document version for per-version
    // output, otherwise the enclosing version module's value.
    let api_version = rest_hir.document.version.clone().or_else(|| {
        let module = module_path.last()?;
        rest_hir.document.version_values.get(module).cloned()
    });
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
                version_header: rest_hir.document.version_header.clone(),
                api_version,
            },
        )?,
    );
    let (ws_client_helpers, ws_server_helpers) =
        websocket_helpers::render_websocket_helpers(&methods);
    out.client.extend(ws_client_helpers);
    out.server.extend(ws_server_helpers);
    let version_header = rest_hir.document.version_header.clone();
    out.server.push(
        ServerClass::new(&def.header.ident, module_path, methods, version_header)
            .render(renderer)?,
    );

    Ok(out)
}

/// Switched unions whose zod schemas do not yet match the `{tag, data}`
/// wire shape the other generators emit, so direct-union responses keep
/// raw passthrough instead of schema validation.
fn switched_union_idents(spec: &hir::Specification) -> std::collections::BTreeSet<String> {
    fn walk(defs: &[hir::Definition], unions: &mut std::collections::BTreeSet<String>) {
        for def in defs {
            match def {
                hir::Definition::ModuleDcl(module) => walk(&module.definition, unions),
                hir::Definition::ConstrTypeDcl(constr) => collect(constr, unions),
                hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(constr)) => {
                    collect(constr, unions);
                }
                _ => {}
            }
        }
    }

    fn collect(constr: &hir::ConstrTypeDcl, unions: &mut std::collections::BTreeSet<String>) {
        if let hir::ConstrTypeDcl::UnionDef(def) = constr {
            unions.insert(ts_ident(&def.ident));
        }
    }

    let mut unions = std::collections::BTreeSet::new();
    walk(&spec.0, &mut unions);
    unions
}
