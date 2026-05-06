use std::fmt::Write;
use xidl_parser::rest_hir::semantics::HttpStreamCodec;

use super::spec_context::{MethodContext, ParamContext, ParamSource};
use super::spec_types::py_bool;

pub(super) fn render_request_value(out: &mut String, method: &MethodContext) {
    if method.request_params.is_empty() {
        writeln!(out, "    request_value = {}()", method.request_type).unwrap();
        return;
    }
    writeln!(out, "    request_value = {}(", method.request_type).unwrap();
    for param in &method.request_params {
        writeln!(
            out,
            "        {}={},",
            param.field_name,
            render_param_binding(param)
        )
        .unwrap();
    }
    writeln!(out, "    )").unwrap();
}

pub(super) fn stream_codec_name(value: HttpStreamCodec) -> &'static str {
    match value {
        HttpStreamCodec::Sse => "sse",
        HttpStreamCodec::Ndjson => "ndjson",
    }
}

fn render_param_binding(param: &ParamContext) -> String {
    match param.source {
        ParamSource::Path => scalar_binding("path_value", param),
        ParamSource::Query => scalar_binding("query_value", param),
        ParamSource::Header => scalar_binding("header_value", param),
        ParamSource::Cookie => scalar_binding("cookie_value", param),
        ParamSource::Body if param.flatten => format!(
            "read_json_value(body, {:?}, optional={}, wire_name={:?})",
            param.ty,
            py_bool(param.optional),
            param.wire_name
        ),
        ParamSource::Body => format!(
            "read_json_field(body, {:?}, {:?}, optional={})",
            param.wire_name,
            param.ty,
            py_bool(param.optional)
        ),
    }
}

fn scalar_binding(read_fn: &str, param: &ParamContext) -> String {
    format!(
        "read_scalar({read_fn}(request, {:?}), {:?}, optional={}, default_on_missing=False, wire_name={:?})",
        param.wire_name,
        param.ty,
        py_bool(param.optional),
        param.wire_name
    )
}
