use crate::error::IdlcResult;
use crate::generate::go_rest::MethodMeta;
use std::fmt::Write;
use xidl_parser::rest_hir::semantics::{HttpStreamCodec, HttpStreamKind};

use super::GoRestRenderer;
use super::interface_binding::render_response_write;

pub(super) fn render_accept_check(out: &mut String, method: &MethodMeta) {
    writeln!(
        out,
        "\t\tif err := xidlgohttp.GinRequireAccept(c, \"{}\"); err != nil {{",
        method.response_content_type
    )
    .unwrap();
    writeln!(out, "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusNotAcceptable, \"NOT_ACCEPTABLE\", err.Error())").unwrap();
    writeln!(out, "\t\t\treturn").unwrap();
    writeln!(out, "\t\t}}").unwrap();
}

pub(super) fn render_auth_check(out: &mut String, method: &MethodMeta) {
    if method.security.is_empty() {
        return;
    }
    writeln!(
        out,
        "\t\tctx, err := xidlgohttp.RequireAuth(c.Request, {}SecurityRequirements())",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\t\tif err != nil {{").unwrap();
    writeln!(
        out,
        "\t\t\txidlgohttp.Unauthorized(c.Writer, {}SecurityRequirements())",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\t\t\treturn").unwrap();
    writeln!(out, "\t\t}}").unwrap();
    writeln!(out, "\t\tc.Request = c.Request.WithContext(ctx)").unwrap();
}

pub(super) fn render_content_type_check(out: &mut String, method: &MethodMeta) {
    if method.request_content_type == "application/json"
        && method.body_params.is_empty()
        && !matches!(method.stream_kind, Some(HttpStreamKind::Client))
    {
        return;
    }
    let expected = if matches!(method.stream_kind, Some(HttpStreamKind::Client)) {
        "application/x-ndjson"
    } else {
        method.request_content_type.as_str()
    };
    writeln!(
        out,
        "\t\tif err := xidlgohttp.GinRequireContentType(c, \"{}\"); err != nil {{",
        expected
    )
    .unwrap();
    writeln!(out, "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusUnsupportedMediaType, \"UNSUPPORTED_MEDIA_TYPE\", err.Error())").unwrap();
    writeln!(out, "\t\t\treturn").unwrap();
    writeln!(out, "\t\t}}").unwrap();
}

pub(super) fn render_server_stream_handler(out: &mut String, method: &MethodMeta) {
    let item_ty = method
        .return_ty
        .clone()
        .unwrap_or_else(|| "string".to_string());
    let ctor = if method.stream_codec == HttpStreamCodec::Sse {
        "NewSSEServerStreamWriter"
    } else {
        "NewNDJSONServerStreamWriter"
    };
    writeln!(out, "\t\tstream := xidlgohttp.{ctor}[{item_ty}](c.Writer)").unwrap();
    writeln!(
        out,
        "\t\tif err := svc.{}(c.Request.Context(), req, stream); err != nil {{",
        method.method_name
    )
    .unwrap();
    writeln!(out, "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusInternalServerError, \"INTERNAL\", err.Error())").unwrap();
    writeln!(out, "\t\t\treturn").unwrap();
    writeln!(out, "\t\t}}").unwrap();
    writeln!(out, "\t\t_ = stream.Close()").unwrap();
}

pub(super) fn render_client_stream_handler(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    writeln!(
        out,
        "\t\tstream := xidlgohttp.NewClientStreamReader[{}](c.Request.Context(), c.Request.Body)",
        method.request_struct
    )
    .unwrap();
    writeln!(
        out,
        "\t\tresp, err := svc.{}(c.Request.Context(), stream)",
        method.method_name
    )
    .unwrap();
    writeln!(out, "\t\tif err != nil {{").unwrap();
    writeln!(out, "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusInternalServerError, \"INTERNAL\", err.Error())").unwrap();
    writeln!(out, "\t\t\treturn").unwrap();
    writeln!(out, "\t\t}}").unwrap();
    render_response_write(out, method, "resp", renderer)
}

pub(super) fn render_unary_handler(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    if method.response_body_struct.is_some()
        || method.response_body_direct_field.is_some()
        || !method.response_header_params.is_empty()
        || !method.response_cookie_params.is_empty()
    {
        writeln!(
            out,
            "\t\tresp, err := svc.{}(c.Request.Context(), req)",
            method.method_name
        )
        .unwrap();
        writeln!(out, "\t\tif err != nil {{").unwrap();
        writeln!(out, "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusInternalServerError, \"INTERNAL\", err.Error())").unwrap();
        writeln!(out, "\t\t\treturn").unwrap();
        writeln!(out, "\t\t}}").unwrap();
        render_response_write(out, method, "resp", renderer)?;
    } else {
        writeln!(
            out,
            "\t\tif _, err := svc.{}(c.Request.Context(), req); err != nil {{",
            method.method_name
        )
        .unwrap();
        writeln!(out, "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusInternalServerError, \"INTERNAL\", err.Error())").unwrap();
        writeln!(out, "\t\t\treturn").unwrap();
        writeln!(out, "\t\t}}").unwrap();
        writeln!(out, "\t\tc.Status(http.StatusNoContent)").unwrap();
    }
    Ok(())
}
