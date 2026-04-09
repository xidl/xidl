use crate::error::{IdlcError, IdlcResult};
use crate::generate::go_http::{MethodMeta, definition};
use crate::generate::http_hir::semantics::HttpStreamKind;
use std::collections::HashMap;
use std::fmt::Write;

use super::GoHttpRenderer;
use super::interface_binding::render_request_binding;
use super::interface_server_support::{
    render_accept_check, render_auth_check, render_client_stream_handler,
    render_content_type_check, render_server_stream_handler, render_unary_handler,
};

pub(super) fn render_server(
    out: &mut String,
    interface_name: &str,
    methods: &[MethodMeta],
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    writeln!(
        out,
        "func New{interface_name}Handler(svc {interface_name}Service) http.Handler {{"
    )
    .unwrap();
    writeln!(out, "\tmux := http.NewServeMux()").unwrap();
    let mut seen_routes = HashMap::<String, String>::new();
    for method in methods {
        for path in &method.paths {
            register_route(out, method, path, &mut seen_routes, renderer)?;
        }
    }
    writeln!(out, "\treturn mux").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn register_route(
    out: &mut String,
    method: &MethodMeta,
    path: &str,
    seen_routes: &mut HashMap<String, String>,
    renderer: &GoHttpRenderer,
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

    writeln!(
        out,
        "\tmux.HandleFunc(\"{} {}\", func(w http.ResponseWriter, r *http.Request) {{",
        definition::http_method_name(method.http_method),
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
