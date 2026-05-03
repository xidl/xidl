use crate::error::IdlcResult;
use xidl_parser::http_hir::semantics::HttpStreamKind;
use std::fmt::Write;

use super::is_server_stream;
use super::spec_binding::{render_request_value, stream_codec_name};
use super::spec_context::{MethodContext, ParamSource};
use super::spec_types::maybe_optional_type;

pub(super) fn render_method_types(out: &mut String, method: &MethodContext) -> IdlcResult<()> {
    writeln!(out, "@dataclass").unwrap();
    writeln!(out, "class {}:", method.request_type).unwrap();
    if method.request_params.is_empty() {
        writeln!(out, "    pass").unwrap();
    } else {
        for param in &method.request_params {
            writeln!(
                out,
                "    {}: {}",
                param.field_name,
                maybe_optional_type(param.optional, &param.ty)
            )
            .unwrap();
        }
    }
    writeln!(out).unwrap();

    if is_server_stream(method.stream_kind) {
        return Ok(());
    }

    writeln!(out, "@dataclass").unwrap();
    writeln!(out, "class {}:", method.response_type).unwrap();
    let response_field_count =
        method.response_params.len() + usize::from(method.return_ty.is_some());
    if response_field_count == 0 {
        writeln!(out, "    pass").unwrap();
    } else if response_field_count == 1 && method.return_ty.is_some() {
        writeln!(
            out,
            "    value: {}",
            method.return_ty.as_deref().unwrap_or("Any")
        )
        .unwrap();
    } else {
        if let Some(return_ty) = &method.return_ty {
            writeln!(out, "    return_: {}", return_ty).unwrap();
        }
        for param in &method.response_params {
            writeln!(
                out,
                "    {}: {}",
                param.field_name,
                maybe_optional_type(param.optional, &param.ty)
            )
            .unwrap();
        }
    }
    writeln!(out).unwrap();
    Ok(())
}

pub(super) fn render_endpoint_helper(
    out: &mut String,
    interface_name: &str,
    method: &MethodContext,
) -> IdlcResult<()> {
    writeln!(
        out,
        "async def {}(service: {}Service, request: Request):",
        method.endpoint_name, interface_name
    )
    .unwrap();
    writeln!(
        out,
        "    require_accept(request, {:?})",
        method.response_content_type
    )
    .unwrap();
    if method.requires_request_content_type {
        writeln!(
            out,
            "    require_content_type(request, {:?})",
            method.request_content_type
        )
        .unwrap();
    }
    if method.security_expr != "[]" {
        writeln!(
            out,
            "    _security = require_security(request, {})",
            method.security_expr
        )
        .unwrap();
    }
    if method
        .request_params
        .iter()
        .any(|param| matches!(param.source, ParamSource::Body))
    {
        writeln!(out, "    body = read_json_body(request)").unwrap();
    }
    render_request_value(out, method);
    writeln!(
        out,
        "    response_value = await service.{}(request_value)",
        method.method_name
    )
    .unwrap();
    match method.stream_kind {
        Some(HttpStreamKind::Server) => {
            writeln!(
                out,
                "    return encode_stream_response(response_value, codec={:?})",
                stream_codec_name(method.stream_codec)
            )
            .unwrap();
        }
        _ => writeln!(out, "    return encode_json_response(response_value)").unwrap(),
    }
    writeln!(out).unwrap();
    Ok(())
}

pub(super) fn render_route_builder(
    out: &mut String,
    interface_name: &str,
    method: &MethodContext,
) -> IdlcResult<()> {
    writeln!(
        out,
        "def {}(service: {}Service) -> Route:",
        method.route_builder_name, interface_name
    )
    .unwrap();
    writeln!(out, "    async def endpoint(request: Request):").unwrap();
    writeln!(
        out,
        "        return await {}(service, request)",
        method.endpoint_name
    )
    .unwrap();
    writeln!(out, "    return Route(").unwrap();
    writeln!(out, "        method={:?},", method.http_method).unwrap();
    writeln!(out, "        paths={:?},", method.paths).unwrap();
    writeln!(out, "        endpoint=endpoint,").unwrap();
    writeln!(out, "        handler=service.{},", method.method_name).unwrap();
    writeln!(out, "        request_model={},", method.request_type).unwrap();
    writeln!(out, "        response_model={},", method.response_type).unwrap();
    writeln!(
        out,
        "        metadata=RouteMetadata(request_content_type={:?}, response_content_type={:?}, security={}, stream={}),",
        method.request_content_type, method.response_content_type, method.security_expr, method.stream_expr
    )
    .unwrap();
    writeln!(out, "    )").unwrap();
    writeln!(out).unwrap();
    Ok(())
}
