use crate::error::IdlcResult;
use crate::generate::go_rest::{MethodMeta, definition};
use serde::Serialize;
use xidl_parser::rest_hir::semantics::HttpStreamKind;

use super::GoRestRenderer;
use super::interface_binding::render_response_write;

#[derive(Serialize)]
struct UnaryHandlerTemplate<'a> {
    binding: &'static str,
    method_name: &'a str,
    server_error: String,
    has_response: bool,
    response_write: String,
}

#[derive(Serialize)]
struct BidiHandlerTemplate<'a> {
    has_handshake: bool,
    request_struct: &'a str,
    binding_lines: Vec<String>,
    subproto: String,
    stream_in_ty: &'a str,
    stream_out_ty: &'a str,
    method_name: &'a str,
    args: String,
}

pub(super) fn content_type_check_target(method: &MethodMeta) -> Option<&str> {
    if !method.request_content_type_explicit {
        return None;
    }
    if method.body_params.is_empty() && !matches!(method.stream_kind, Some(HttpStreamKind::Client))
    {
        return None;
    }
    let expected = if matches!(method.stream_kind, Some(HttpStreamKind::Client)) {
        "application/x-ndjson"
    } else {
        method.request_content_type.as_str()
    };
    (!expected.is_empty()).then_some(expected)
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
    let mut server_error = String::new();
    super::exceptions::ExceptionCodec::new(method)
        .render_server_error(&mut server_error, renderer)?;
    let mut response_write = String::new();
    if has_response {
        render_response_write(&mut response_write, method, "resp", renderer)?;
    }
    out.push_str(&renderer.render_template(
        "server_unary_handler.go.j2",
        &UnaryHandlerTemplate {
            binding,
            method_name: &method.method_name,
            server_error,
            has_response,
            response_write,
        },
    )?);
    out.push('\n');
    Ok(())
}

pub(super) fn render_bidi_stream_handler(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let mut binding_lines = Vec::new();
    if !method.handshake_params.is_empty() {
        let mut bindings = String::new();
        for param in &method.handshake_params {
            definition::emit_bidi_request_bind(&mut bindings, param)?;
        }
        binding_lines = bindings.lines().map(str::to_owned).collect();
    }
    let subproto = method
        .websocket_subprotocol
        .as_deref()
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "\"\"".to_string());
    let mut args = String::from("c.Request.Context(), stream");
    for param in &method.handshake_params {
        args.push_str(&format!(", req.{}", param.field_name));
    }
    out.push_str(&renderer.render_template(
        "server_bidi_handler.go.j2",
        &BidiHandlerTemplate {
            has_handshake: !method.handshake_params.is_empty(),
            request_struct: &method.request_struct,
            binding_lines,
            subproto,
            stream_in_ty: &method.stream_in_ty,
            stream_out_ty: &method.stream_out_ty,
            method_name: &method.method_name,
            args,
        },
    )?);
    out.push('\n');
    Ok(())
}
