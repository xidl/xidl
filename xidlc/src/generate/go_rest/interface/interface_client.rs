use crate::error::IdlcResult;
use crate::generate::go_rest::{MethodMeta, definition};
use serde::Serialize;
use xidl_parser::hir::HttpParameterLocation;
use xidl_parser::rest_hir::semantics::HttpStreamKind;

use super::GoRestRenderer;
use super::interface_binding::render_client_build_request;

#[derive(Serialize)]
struct ClientHeaderTemplate<'a> {
    interface_name: &'a str,
    version_value: Option<&'a str>,
}

#[derive(Serialize)]
struct ClientUnaryTemplate<'a> {
    interface_name: &'a str,
    method_name: &'a str,
    request_struct: &'a str,
    response_struct: &'a str,
    struct_prefix: &'a str,
    build_request: String,
    http_error: String,
}

#[derive(Serialize)]
struct VersionBindingTemplate<'a> {
    name: &'a str,
    value: &'a str,
    is_cookie: bool,
}

#[derive(Serialize)]
struct ClientStreamTemplate<'a> {
    interface_name: &'a str,
    method_name: &'a str,
    request_struct: &'a str,
    response_struct: &'a str,
    struct_prefix: &'a str,
    path: &'a str,
    http_method: &'a str,
    response_content_type: &'a str,
    version_query: Option<VersionBindingTemplate<'a>>,
    version_header: Option<VersionBindingTemplate<'a>>,
    version_cookie: Option<VersionBindingTemplate<'a>>,
    http_error: String,
}

#[derive(Serialize)]
struct ClientBidiTemplate<'a> {
    interface_name: &'a str,
    method_name: &'a str,
    params: String,
    stream_in_ty: &'a str,
    stream_out_ty: &'a str,
    request_struct: &'a str,
    req_fields: String,
    struct_prefix: &'a str,
    has_query: bool,
    query_lines: Vec<String>,
    version_query: Option<VersionBindingTemplate<'a>>,
    subproto: String,
    ws_version_header: Option<VersionBindingTemplate<'a>>,
}

pub(super) fn render_client(
    out: &mut String,
    interface_name: &str,
    methods: &[MethodMeta],
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let version_value = methods
        .iter()
        .find_map(|method| method.version_parameter.as_ref())
        .map(|version| version.value.as_str());
    out.push_str(&renderer.render_template(
        "client_header.go.j2",
        &ClientHeaderTemplate {
            interface_name,
            version_value,
        },
    )?);
    out.push_str("\n\n");

    for method in methods {
        match method.stream_kind {
            Some(HttpStreamKind::Server) => {
                render_client_server_stream(out, interface_name, method, renderer)?
            }
            Some(HttpStreamKind::Client) => {
                render_client_client_stream(out, interface_name, method, renderer)?
            }
            Some(HttpStreamKind::Bidi) => {
                render_client_bidi_stream(out, interface_name, method, renderer)?
            }
            None => render_client_unary(out, interface_name, method, renderer)?,
        }
    }
    Ok(())
}

fn render_client_unary(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let mut build_request = String::new();
    render_client_build_request(&mut build_request, method, renderer)?;
    let mut http_error = String::new();
    super::exceptions::ExceptionCodec::new(method).render_http_error(&mut http_error, renderer)?;
    out.push_str(&renderer.render_template(
        "client_unary.go.j2",
        &ClientUnaryTemplate {
            interface_name,
            method_name: &method.method_name,
            request_struct: &method.request_struct,
            response_struct: &method.response_struct,
            struct_prefix: &method.struct_prefix,
            build_request,
            http_error,
        },
    )?);
    out.push_str("\n\n");
    Ok(())
}

fn render_client_server_stream(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    super::stream::StreamMethod::new(method).render_client(out, interface_name, renderer)
}

fn render_client_client_stream(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let mut http_error = String::new();
    super::exceptions::ExceptionCodec::new(method).render_http_error(&mut http_error, renderer)?;
    let version = method.version_parameter.as_ref();
    let version_binding = |loc| {
        version
            .filter(|parameter| parameter.location == loc)
            .map(|parameter| VersionBindingTemplate {
                name: &parameter.name,
                value: &parameter.value,
                is_cookie: loc == HttpParameterLocation::Cookie,
            })
    };
    out.push_str(&renderer.render_template(
        "client_client_stream.go.j2",
        &ClientStreamTemplate {
            interface_name,
            method_name: &method.method_name,
            request_struct: &method.request_struct,
            response_struct: &method.response_struct,
            struct_prefix: &method.struct_prefix,
            path: method.paths.first().map(String::as_str).unwrap_or("/"),
            http_method: definition::http_method_name(method.http_method),
            response_content_type: &method.response_content_type,
            version_query: version_binding(HttpParameterLocation::Query),
            version_header: version_binding(HttpParameterLocation::Header),
            version_cookie: version_binding(HttpParameterLocation::Cookie),
            http_error,
        },
    )?);
    out.push_str("\n\n");
    Ok(())
}

fn render_client_bidi_stream(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let mut query_encode = String::new();
    for param in &method.query_params {
        definition::emit_query_encode(&mut query_encode, param)?;
    }
    let mut params = vec!["ctx context.Context".to_string()];
    let mut req_fields = Vec::new();
    for param in &method.handshake_params {
        let ty = if param.optional {
            format!("*{}", param.ty)
        } else {
            param.ty.clone()
        };
        let name = param.field_name.to_ascii_lowercase();
        params.push(format!("{name} {ty}"));
        req_fields.push(format!("{}: {name},", param.field_name));
    }
    let version_query = method
        .version_parameter
        .as_ref()
        .filter(|parameter| parameter.location == HttpParameterLocation::Query)
        .map(|parameter| VersionBindingTemplate {
            name: &parameter.name,
            value: &parameter.value,
            is_cookie: false,
        });
    let ws_version_header = method
        .version_parameter
        .as_ref()
        .filter(|parameter| {
            matches!(
                parameter.location,
                HttpParameterLocation::Header | HttpParameterLocation::Cookie
            )
        })
        .map(|parameter| VersionBindingTemplate {
            name: &parameter.name,
            value: &parameter.value,
            is_cookie: parameter.location == HttpParameterLocation::Cookie,
        });
    let subproto = method
        .websocket_subprotocol
        .as_deref()
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "\"\"".to_string());
    let query_lines: Vec<String> = query_encode.lines().map(str::to_owned).collect();
    out.push_str(&renderer.render_template(
        "client_bidi_stream.go.j2",
        &ClientBidiTemplate {
            interface_name,
            method_name: &method.method_name,
            params: params.join(", "),
            stream_in_ty: &method.stream_in_ty,
            stream_out_ty: &method.stream_out_ty,
            request_struct: &method.request_struct,
            req_fields: req_fields.join(" "),
            struct_prefix: &method.struct_prefix,
            has_query: !query_lines.is_empty() || version_query.is_some(),
            query_lines,
            version_query,
            subproto,
            ws_version_header,
        },
    )?);
    out.push_str("\n\n");
    Ok(())
}
