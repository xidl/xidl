use super::model::{ClientClassContext, MethodModel, TsHttpBlocks};
use super::server::ServerClass;
use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::names::ts_ident;
use method::OperationModelInput;
use render_types::{render_request_types, render_response_types};
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

mod client;
mod exceptions;
pub(super) mod helpers;
mod method;
mod render_types;
mod representations;
mod response_schema;
pub(crate) mod text_aliases;
mod websocket_helpers;

pub(crate) fn render_interface(
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
    rest_hir: &RestHirDocument,
    text_aliases: &text_aliases::TextAliases,
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
                text_aliases,
            })
        })
        .collect::<IdlcResult<Vec<_>>>()?;
    let mut out = TsHttpBlocks::default();
    for method in &methods {
        render_request_types(&mut out, method, module_path, renderer)?;
        render_response_types(&mut out, method, module_path, renderer)?;
    }
    let api_version = rest_hir
        .document
        .api_version(module_path)
        .map(str::to_string);
    let version_parameter = rest_hir.document.version_parameter.clone();
    out.client.push(client::render(
        renderer,
        &ClientClassContext {
            client_name: ts_ident(&def.header.ident), // Template adds 'Client'
            methods: methods
                .iter()
                .cloned()
                .map(MethodModel::into_client_context)
                .collect(),
            version_parameter: version_parameter.clone(),
            api_version: api_version.clone(),
            cookie_version: version_parameter
                .as_ref()
                .is_some_and(|parameter| parameter.location == hir::HttpParameterLocation::Cookie),
        },
        version_parameter.as_ref().zip(api_version.as_deref()),
    )?);
    let (ws_client_helpers, ws_server_helpers) = websocket_helpers::WebsocketHelpers::render(
        &methods,
        renderer,
        version_parameter.as_ref().zip(api_version.as_deref()),
    )?;
    out.client.extend(ws_client_helpers);
    out.server.extend(ws_server_helpers);
    let fixed_response_headers = version_parameter
        .as_ref()
        .zip(api_version.as_ref())
        .filter(|(parameter, _)| parameter.location == hir::HttpParameterLocation::Header)
        .map(|(parameter, value)| (parameter.name.clone(), value.clone()))
        .into_iter()
        .collect();
    out.server.push(
        ServerClass::new(
            &def.header.ident,
            module_path,
            methods,
            fixed_response_headers,
        )?
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
