use crate::error::IdlcResult;
use crate::generate::go_rest::{MethodMeta, definition};
use std::fmt::Write;
use xidl_parser::rest_hir::semantics::HttpStreamKind;

use super::GoRestRenderer;
use super::interface_binding::render_response_write;

pub(super) fn render_accept_check(out: &mut String, method: &MethodMeta) {
    if !method.response_content_type_explicit {
        return;
    }
    writeln!(
        out,
        "\t\tif err := xidlgohttp.GinRequireAccept(c, \"{}\"); err != nil {{",
        method.response_content_type
    )
    .unwrap();
    writeln!(out, "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusNotAcceptable, http.StatusNotAcceptable, err.Error())").unwrap();
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
    if !method.request_content_type_explicit {
        return;
    }
    if method.body_params.is_empty() && !matches!(method.stream_kind, Some(HttpStreamKind::Client))
    {
        return;
    }
    let expected = if matches!(method.stream_kind, Some(HttpStreamKind::Client)) {
        "application/x-ndjson"
    } else {
        method.request_content_type.as_str()
    };
    if expected.is_empty() {
        return;
    }
    writeln!(
        out,
        "\t\tif err := xidlgohttp.GinRequireContentType(c, \"{}\"); err != nil {{",
        expected
    )
    .unwrap();
    writeln!(out, "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusUnsupportedMediaType, http.StatusUnsupportedMediaType, err.Error())").unwrap();
    writeln!(out, "\t\t\treturn").unwrap();
    writeln!(out, "\t\t}}").unwrap();
}

pub(super) fn render_server_stream_handler(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    super::stream::StreamMethod::new(method).render_server(out, renderer)
}

pub(super) fn render_client_stream_handler(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    super::stream::StreamMethod::new(method).render_upload(out, renderer)?;
    render_response_write(out, method, "resp", renderer)
}

pub(super) fn render_unary_handler(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let has_response = method.response_body_struct.is_some()
        || method.response_body_direct_field.is_some()
        || !method.response_header_params.is_empty()
        || !method.response_cookie_params.is_empty();
    let binding = if has_response { "resp" } else { "_" };
    writeln!(
        out,
        "\t\t{binding}, err := svc.{}(c.Request.Context(), req)",
        method.method_name
    )
    .unwrap();
    super::exceptions::ExceptionCodec::new(method).render_server_error(out, renderer)?;
    if has_response {
        render_response_write(out, method, "resp", renderer)?;
    } else {
        writeln!(out, "\t\tc.Status(http.StatusNoContent)").unwrap();
    }
    Ok(())
}

pub(super) fn render_bidi_stream_handler(
    out: &mut String,
    method: &MethodMeta,
    _renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    if !method.handshake_params.is_empty() {
        writeln!(out, "\t\treq := &{}{{}}", method.request_struct).unwrap();
        let mut bindings = String::new();
        for param in &method.handshake_params {
            definition::emit_bidi_request_bind(&mut bindings, param)?;
        }
        for line in bindings.lines() {
            writeln!(out, "\t{line}").unwrap();
        }
    }
    let subproto = method
        .websocket_subprotocol
        .as_deref()
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "\"\"".to_string());
    writeln!(out, "\t\tvar subprotocols []string").unwrap();
    writeln!(
        out,
        "\t\tif {subproto} != \"\" {{ subprotocols = []string{{{subproto}}} }}"
    )
    .unwrap();
    writeln!(
        out,
        "\t\tconn, err := xidlgohttp.UpgradeWebSocket(c, subprotocols)"
    )
    .unwrap();
    writeln!(out, "\t\tif err != nil {{").unwrap();
    writeln!(
        out,
        "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusBadRequest, http.StatusBadRequest, err.Error())"
    )
    .unwrap();
    writeln!(out, "\t\t\treturn").unwrap();
    writeln!(out, "\t\t}}").unwrap();
    writeln!(
        out,
        "\t\tstream := xidlgohttp.NewWSBidiServer[{}, {}](conn)",
        method.stream_in_ty, method.stream_out_ty
    )
    .unwrap();
    let mut args = String::from("c.Request.Context(), stream");
    for param in &method.handshake_params {
        args.push_str(&format!(", req.{}", param.field_name));
    }
    writeln!(
        out,
        "\t\tif err := svc.{}({args}); err != nil {{",
        method.method_name
    )
    .unwrap();
    writeln!(
        out,
        "\t\t\txidlgohttp.GinWriteJSONError(c, http.StatusInternalServerError, http.StatusInternalServerError, err.Error())"
    )
    .unwrap();
    writeln!(out, "\t\t}}").unwrap();
    writeln!(out, "\t\t_ = stream.Close()").unwrap();
    Ok(())
}
