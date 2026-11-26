use crate::error::IdlcResult;
use crate::generate::go_rest::{HttpMethod, MethodMeta, definition};
use serde::Serialize;
use xidl_parser::hir::HttpParameterLocation;
use xidl_parser::rest_hir::semantics::{HttpCorsProfile, HttpStreamKind};

use super::GoRestRenderer;
use super::interface_binding::render_request_binding;
use super::interface_server_support::{
    content_type_check_target, render_bidi_stream_handler, render_client_stream_handler,
    render_server_stream_handler, render_unary_handler,
};

#[derive(Serialize)]
struct ServerRegisterTemplate<'a> {
    interface_name: &'a str,
    routes: Vec<ServerRouteTemplate<'a>>,
}

#[derive(Serialize)]
struct VersionHeaderTemplate<'a> {
    name: &'a str,
    value: &'a str,
}

#[derive(Serialize)]
struct ServerRouteTemplate<'a> {
    gin_method: &'static str,
    pattern: String,
    cors_mw: Option<String>,
    version_header: Option<VersionHeaderTemplate<'a>>,
    accept_check: Option<&'a str>,
    has_security: bool,
    struct_prefix: &'a str,
    content_type_check: Option<&'a str>,
    binding: String,
    handler: String,
}

pub(super) fn render_server(
    out: &mut String,
    interface_name: &str,
    methods: &[MethodMeta],
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let mut routes = Vec::new();
    for method in methods {
        for path in &method.paths {
            routes.push(build_route_template(method, path, renderer)?);
        }
    }
    out.push_str(&renderer.render_template(
        "server_register.go.j2",
        &ServerRegisterTemplate {
            interface_name,
            routes,
        },
    )?);
    out.push_str("\n\n");
    Ok(())
}

fn build_route_template<'a>(
    method: &'a MethodMeta,
    path: &str,
    renderer: &GoRestRenderer,
) -> IdlcResult<ServerRouteTemplate<'a>> {
    let gin_method = match method.http_method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    };

    let cors_mw = method.cors.as_ref().map(|cors| {
        let origins = match cors {
            HttpCorsProfile::Any => vec!["*".to_string()],
            HttpCorsProfile::Origins(o) => o.clone(),
        };
        format!(
            "xidlgohttp.CORSMiddleware([]string{{{}}})",
            origins
                .iter()
                .map(|o| format!("{o:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    });

    let version_header = method
        .version_parameter
        .as_ref()
        .filter(|parameter| parameter.location == HttpParameterLocation::Header)
        .map(|parameter| VersionHeaderTemplate {
            name: &parameter.name,
            value: &parameter.value,
        });

    let is_bidi = matches!(method.stream_kind, Some(HttpStreamKind::Bidi));
    let accept_check = (!is_bidi && method.response_content_type_explicit)
        .then_some(method.response_content_type.as_str());
    let content_type_check = content_type_check_target(method);

    let mut binding = String::new();
    if !is_bidi {
        render_request_binding(&mut binding, method, renderer)?;
    }
    let mut handler = String::new();
    match method.stream_kind {
        Some(HttpStreamKind::Server) => {
            render_server_stream_handler(&mut handler, method, renderer)?
        }
        Some(HttpStreamKind::Client) => {
            render_client_stream_handler(&mut handler, method, renderer)?
        }
        Some(HttpStreamKind::Bidi) => render_bidi_stream_handler(&mut handler, method, renderer)?,
        None => render_unary_handler(&mut handler, method, renderer)?,
    }

    Ok(ServerRouteTemplate {
        gin_method,
        pattern: definition::go_pattern_path(path),
        cors_mw,
        version_header,
        accept_check,
        has_security: !method.security.is_empty(),
        struct_prefix: &method.struct_prefix,
        content_type_check,
        binding,
        handler,
    })
}
