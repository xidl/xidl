use crate::error::{IdlcError, IdlcResult};
use crate::generate::go_rest::{HttpMethod, MethodMeta, definition};
use std::collections::HashMap;
use std::fmt::Write;
use xidl_parser::rest_hir::semantics::HttpStreamKind;

use super::GoRestRenderer;
use super::interface_binding::render_request_binding;
use super::interface_server_support::{
    render_accept_check, render_auth_check, render_client_stream_handler,
    render_content_type_check, render_server_stream_handler, render_unary_handler,
};

pub(super) fn render_server(
    out: &mut String,
    interface_name: &str,
    methods: &[MethodMeta],
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    writeln!(
        out,
        "func Register{interface_name}Handler(r gin.IRouter, svc {interface_name}Service) {{"
    )
    .unwrap();
    let mut seen_routes = HashMap::<String, String>::new();
    for method in methods {
        for path in &method.paths {
            register_route(out, method, path, &mut seen_routes, renderer)?;
        }
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn register_route(
    out: &mut String,
    method: &MethodMeta,
    path: &str,
    seen_routes: &mut HashMap<String, String>,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let route_key = format!(
        "{} {}",
        definition::http_method_name(method.http_method),
        path
    );
    if let Some(previous) = seen_routes.insert(route_key.clone(), method.method_name.clone()) {
        return Err(IdlcError::rpc(format!(
            "duplicate HTTP route binding: {route_key} (methods: {previous}, {})",
            method.method_name
        )));
    }

    let gin_method = match method.http_method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    };

    writeln!(
        out,
        "\tr.{}(\"{}\", func(c *gin.Context) {{",
        gin_method,
        definition::go_pattern_path(path)
    )
    .unwrap();
    render_accept_check(out, method);
    render_auth_check(out, method);
    render_content_type_check(out, method);
    render_request_binding(out, method, renderer)?;
    match method.stream_kind {
        Some(HttpStreamKind::Server) => render_server_stream_handler(out, method),
        Some(HttpStreamKind::Client) => render_client_stream_handler(out, method, renderer)?,
        Some(HttpStreamKind::Bidi) => {}
        None => render_unary_handler(out, method, renderer)?,
    }
    writeln!(out, "\t}})").unwrap();
    Ok(())
}
