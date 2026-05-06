mod interface_binding;
mod interface_client;
mod interface_meta;
mod interface_meta_support;
mod interface_server;
mod interface_server_support;
mod interface_templates;
mod interface_types;

use crate::error::IdlcResult;
use crate::generate::go_rest::MethodMeta;
use std::fmt::Write;
use xidl_parser::hir;
use xidl_parser::rest_hir::{HttpOperationSource, RestHirDocument};

use super::GoRestRenderer;
use interface_client::render_client;
pub(crate) use interface_meta::build_method_meta;
use interface_server::render_server;
use interface_types::render_method_types;

pub(crate) fn render_interface(
    out: &mut String,
    interface: &hir::InterfaceDcl,
    prefix: &[String],
    renderer: &GoRestRenderer,
    rest_hir: &RestHirDocument,
) -> IdlcResult<()> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(());
    };
    let interface_name = super::definition::export_name(prefix, &def.header.ident);
    let Some(http_interface) = rest_hir.find_interface(prefix, &def.header.ident) else {
        return Ok(());
    };
    let methods = http_interface
        .operations
        .iter()
        .filter(|operation| matches!(operation.source, HttpOperationSource::Method))
        .map(|operation| build_method_meta(&interface_name, operation))
        .collect::<IdlcResult<Vec<_>>>()?;

    writeln!(out, "type {interface_name}Service interface {{").unwrap();
    for method in &methods {
        render_service_method(out, method);
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    render_server(out, &interface_name, &methods, renderer)?;
    render_client(out, &interface_name, &methods, renderer)?;
    for method in &methods {
        render_method_types(out, method, renderer)?;
    }
    Ok(())
}

fn render_service_method(out: &mut String, method: &MethodMeta) {
    use xidl_parser::rest_hir::semantics::HttpStreamKind;

    match method.stream_kind {
        Some(HttpStreamKind::Server) => writeln!(
            out,
            "\t{}(ctx context.Context, req *{}, stream xidlgohttp.ServerStreamWriter[{}]) error",
            method.method_name,
            method.request_struct,
            method
                .return_ty
                .clone()
                .unwrap_or_else(|| "string".to_string())
        )
        .unwrap(),
        Some(HttpStreamKind::Client) => writeln!(
            out,
            "\t{}(ctx context.Context, stream *xidlgohttp.ClientStreamReader[{}]) (*{}, error)",
            method.method_name, method.request_struct, method.response_struct
        )
        .unwrap(),
        Some(HttpStreamKind::Bidi) => {}
        None => writeln!(
            out,
            "\t{}(ctx context.Context, req *{}) (*{}, error)",
            method.method_name, method.request_struct, method.response_struct
        )
        .unwrap(),
    }
}
