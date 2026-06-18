mod interface_binding;
mod interface_client;
mod interface_meta;
mod interface_meta_support;
mod interface_server;
mod interface_server_support;
mod interface_templates;
mod interface_types;

use crate::error::IdlcResult;
use crate::generate::go_rest::{GoRestRenderBlocks, MethodMeta};
use std::fmt::Write;
use xidl_parser::hir;
use xidl_parser::rest_hir::HttpOperationSource;

use super::{GoRestRenderContext, GoRestRenderer};
use interface_client::render_client;
pub(crate) use interface_meta::build_method_meta;
use interface_server::render_server;
use interface_types::render_method_types;

pub(crate) fn render_interface(
    interface: &hir::InterfaceDcl,
    prefix: &[String],
    context: &GoRestRenderContext<'_>,
) -> IdlcResult<GoRestRenderBlocks> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(GoRestRenderBlocks::default());
    };
    let interface_name = super::definition::export_name(prefix, &def.header.ident);
    let Some(http_interface) = context.rest_hir.find_interface(prefix, &def.header.ident) else {
        return Ok(GoRestRenderBlocks::default());
    };
    let methods = http_interface
        .operations
        .iter()
        .filter(|operation| {
            matches!(
                operation.meta.source,
                HttpOperationSource::Method
                    | HttpOperationSource::AttributeGet
                    | HttpOperationSource::AttributeSet
            )
        })
        .map(|operation| build_method_meta(&interface_name, operation))
        .collect::<IdlcResult<Vec<_>>>()?;

    let mut blocks = GoRestRenderBlocks::default();

    let mut server = String::new();
    writeln!(server, "type {interface_name}Service interface {{").unwrap();
    for method in &methods {
        render_service_method(&mut server, method);
    }
    writeln!(server, "}}").unwrap();
    writeln!(server).unwrap();
    render_server(&mut server, &interface_name, &methods, context.renderer)?;
    blocks.push_server(server);

    let mut client = String::new();
    render_client(&mut client, &interface_name, &methods, context.renderer)?;
    blocks.push_client(client);

    let mut shared = String::new();
    for method in &methods {
        render_method_types(&mut shared, method, context.renderer)?;
    }
    blocks.push_shared(shared);
    Ok(blocks)
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
