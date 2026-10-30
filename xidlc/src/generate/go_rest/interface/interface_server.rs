use crate::error::IdlcResult;
use crate::generate::go_rest::{HttpMethod, MethodMeta, definition};
use std::fmt::Write;
use xidl_parser::rest_hir::semantics::HttpStreamKind;

use super::GoRestRenderer;
use super::interface_binding::render_request_binding;
use super::interface_server_support::{
    render_accept_check, render_auth_check, render_bidi_stream_handler,
    render_client_stream_handler, render_content_type_check, render_server_stream_handler,
    render_unary_handler,
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
    for method in methods {
        for path in &method.paths {
            register_route(out, method, path, renderer)?;
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
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let gin_method = match method.http_method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    };

    if let Some(cors) = &method.cors {
        use xidl_parser::rest_hir::semantics::HttpCorsProfile;
        let origins = match cors {
            HttpCorsProfile::Any => vec!["*".to_string()],
            HttpCorsProfile::Origins(o) => o.clone(),
        };
        let cors_mw = format!(
            "xidlgohttp.CORSMiddleware([]string{{{}}})",
            origins
                .iter()
                .map(|o| format!("{o:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        );

        writeln!(
            out,
            "\tr.OPTIONS(\"{}\", {cors_mw})",
            definition::go_pattern_path(path)
        )
        .unwrap();

        write!(
            out,
            "\tr.{}(\"{}\", {cors_mw}, ",
            gin_method,
            definition::go_pattern_path(path)
        )
        .unwrap();
    } else {
        write!(
            out,
            "\tr.{}(\"{}\", ",
            gin_method,
            definition::go_pattern_path(path)
        )
        .unwrap();
    }

    writeln!(out, "func(c *gin.Context) {{").unwrap();
    if let Some(version) = method
        .version_parameter
        .as_ref()
        .filter(|parameter| parameter.location == xidl_parser::hir::HttpParameterLocation::Header)
    {
        writeln!(out, "\t\tc.Header({:?}, {:?})", version.name, version.value).unwrap();
    }
    if !matches!(method.stream_kind, Some(HttpStreamKind::Bidi)) {
        render_accept_check(out, method);
    }
    render_auth_check(out, method);
    render_content_type_check(out, method);
    if matches!(method.stream_kind, Some(HttpStreamKind::Bidi)) {
        // WebSocket handshake parameters are bound before the upgrade.
    } else {
        render_request_binding(out, method, renderer)?;
    }
    match method.stream_kind {
        Some(HttpStreamKind::Server) => render_server_stream_handler(out, method, renderer)?,
        Some(HttpStreamKind::Client) => render_client_stream_handler(out, method, renderer)?,
        Some(HttpStreamKind::Bidi) => render_bidi_stream_handler(out, method, renderer)?,
        None => render_unary_handler(out, method, renderer)?,
    }
    writeln!(out, "\t}})").unwrap();
    Ok(())
}
