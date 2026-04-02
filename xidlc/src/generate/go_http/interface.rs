use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::{
    HttpApiKeyLocation, HttpSecurityProfile, HttpSecurityRequirement, HttpStreamCodec,
    HttpStreamKind, HttpStreamTargetSupport, deprecated_info, effective_media_type,
    effective_security_with_origin, http_stream_config, validate_http_annotations,
    validate_http_stream_method, validate_http_stream_target,
};
use convert_case::{Case, Casing};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use xidl_parser::hir;

use super::{GoHttpRenderer, HttpMethod, MethodMeta, ParamDirection, ParamSource};

pub(crate) fn render_interface(
    out: &mut String,
    interface: &hir::InterfaceDcl,
    prefix: &[String],
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(());
    };
    validate_http_annotations(
        &format!("interface '{}'", def.header.ident),
        &interface.annotations,
    )
    .map_err(IdlcError::rpc)?;

    let interface_name = super::definition::export_name(prefix, &def.header.ident);
    let mut methods = Vec::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            if let hir::Export::OpDcl(op) = export {
                methods.push(build_method_meta(
                    &interface_name,
                    &interface.annotations,
                    op,
                )?);
            }
        }
    }

    writeln!(out, "type {interface_name}Service interface {{").unwrap();
    for method in &methods {
        match method.stream_kind {
            Some(HttpStreamKind::Server) => writeln!(
                out,
                "\t{}(ctx context.Context, req *{}, stream xidlgohttp.ServerStreamWriter[{}]) error",
                method.method_name,
                method.request_struct,
                method.return_ty.clone().unwrap_or_else(|| "string".to_string())
            )
            .unwrap(),
            Some(HttpStreamKind::Client) => writeln!(
                out,
                "\t{}(ctx context.Context, stream *xidlgohttp.ClientStreamReader[{}]) (*{}, error)",
                method.method_name, method.request_struct, method.response_struct
            )
            .unwrap(),
            Some(HttpStreamKind::Bidi) => {}
            None => writeln!(
                out,
                "\t{}(ctx context.Context, req *{}) (*{}, error)",
                method.method_name, method.request_struct, method.response_struct
            )
            .unwrap(),
        }
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    render_server(out, &interface_name, &methods, renderer)?;
    render_client(out, &interface_name, &methods, renderer)?;
    for method in &methods {
        render_method_types(out, method, renderer)?;
    }
    Ok(())
}

#[derive(Serialize)]
struct MethodTemplateParam {
    field_name: String,
}

#[derive(Serialize)]
struct ClientBuildRequestTemplate<'a> {
    method: ClientBuildRequestMethod<'a>,
}

#[derive(Serialize)]
struct ClientBuildRequestMethod<'a> {
    struct_prefix: &'a str,
    http_method_name: &'a str,
    request_body_struct: Option<&'a str>,
    request_body_direct_field: Option<&'a str>,
    request_body_direct_ty: Option<&'a str>,
    request_content_type: &'a str,
    response_content_type: &'a str,
    body_params: Vec<MethodTemplateParam>,
    has_query_params: bool,
    has_body_params: bool,
    has_security: bool,
    query_encode: String,
    header_encode: String,
    cookie_encode: String,
}

#[derive(Serialize)]
struct DecodeResponseTemplate<'a> {
    method: DecodeResponseMethod<'a>,
}

#[derive(Serialize)]
struct DecodeResponseMethod<'a> {
    struct_prefix: &'a str,
    response_struct: &'a str,
    response_body_struct: Option<&'a str>,
    response_body_direct_field: Option<&'a str>,
    response_body_direct_ty: Option<&'a str>,
    response_content_type: &'a str,
    return_ty: Option<&'a str>,
    response_body_params: Vec<MethodTemplateParam>,
    response_header_decode: String,
    response_cookie_decode: String,
}

#[derive(Serialize)]
struct RequestBindingTemplate<'a> {
    method: RequestBindingMethod<'a>,
}

#[derive(Serialize)]
struct RequestBindingMethod<'a> {
    is_client_stream: bool,
    request_struct: &'a str,
    request_body_struct: Option<&'a str>,
    request_body_direct_field: Option<&'a str>,
    request_body_direct_ty: Option<&'a str>,
    request_content_type: &'a str,
    body_params: Vec<MethodTemplateParam>,
    path_bindings: String,
    query_bindings: String,
    header_bindings: String,
    cookie_bindings: String,
}

#[derive(Serialize)]
struct ResponseWriteTemplate<'a> {
    method: ResponseWriteMethod<'a>,
    value: &'a str,
}

#[derive(Serialize)]
struct ResponseWriteMethod<'a> {
    response_body_struct: Option<&'a str>,
    response_body_direct_field: Option<&'a str>,
    response_body_direct_ty: Option<&'a str>,
    response_content_type: &'a str,
    return_ty: Option<&'a str>,
    response_body_params: Vec<MethodTemplateParam>,
    response_header_encode: String,
    response_cookie_encode: String,
}

fn render_server(
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
            let route_key = format!(
                "{} {}",
                super::definition::http_method_name(method.http_method),
                path
            );
            if let Some(previous) =
                seen_routes.insert(route_key.clone(), method.method_name.clone())
            {
                return Err(IdlcError::rpc(format!(
                    "duplicate HTTP route binding: {route_key} (methods: {previous}, {})",
                    method.method_name
                )));
            }
            writeln!(
                out,
                "\tmux.HandleFunc(\"{} {}\", func(w http.ResponseWriter, r *http.Request) {{",
                super::definition::http_method_name(method.http_method),
                super::definition::go_pattern_path(path)
            )
            .unwrap();
            writeln!(
                out,
                "\t\tif err := xidlgohttp.RequireAccept(r, \"{}\"); err != nil {{",
                method.response_content_type
            )
            .unwrap();
            writeln!(out, "\t\t\txidlgohttp.WriteJSONError(w, http.StatusNotAcceptable, \"NOT_ACCEPTABLE\", err.Error())").unwrap();
            writeln!(out, "\t\t\treturn").unwrap();
            writeln!(out, "\t\t}}").unwrap();
            if !method.security.is_empty() {
                writeln!(
                    out,
                    "\t\tctx, err := xidlgohttp.RequireAuth(r, {}SecurityRequirements())",
                    method.struct_prefix
                )
                .unwrap();
                writeln!(out, "\t\tif err != nil {{").unwrap();
                writeln!(
                    out,
                    "\t\t\txidlgohttp.Unauthorized(w, {}SecurityRequirements())",
                    method.struct_prefix
                )
                .unwrap();
                writeln!(out, "\t\t\treturn").unwrap();
                writeln!(out, "\t\t}}").unwrap();
                writeln!(out, "\t\tr = r.WithContext(ctx)").unwrap();
            }
            if method.request_content_type != "application/json"
                || !method.body_params.is_empty()
                || matches!(method.stream_kind, Some(HttpStreamKind::Client))
            {
                writeln!(
                    out,
                    "\t\tif err := xidlgohttp.RequireContentType(r, \"{}\"); err != nil {{",
                    if matches!(method.stream_kind, Some(HttpStreamKind::Client)) {
                        "application/x-ndjson"
                    } else {
                        method.request_content_type.as_str()
                    }
                )
                .unwrap();
                writeln!(out, "\t\t\txidlgohttp.WriteJSONError(w, http.StatusUnsupportedMediaType, \"UNSUPPORTED_MEDIA_TYPE\", err.Error())").unwrap();
                writeln!(out, "\t\t\treturn").unwrap();
                writeln!(out, "\t\t}}").unwrap();
            }
            render_request_binding(out, method, renderer)?;
            match method.stream_kind {
                Some(HttpStreamKind::Server) => {
                    if method.stream_codec == HttpStreamCodec::Sse {
                        writeln!(
                            out,
                            "\t\tstream := xidlgohttp.NewSSEServerStreamWriter[{}](w)",
                            method
                                .return_ty
                                .clone()
                                .unwrap_or_else(|| "string".to_string())
                        )
                        .unwrap();
                    } else {
                        writeln!(
                            out,
                            "\t\tstream := xidlgohttp.NewNDJSONServerStreamWriter[{}](w)",
                            method
                                .return_ty
                                .clone()
                                .unwrap_or_else(|| "string".to_string())
                        )
                        .unwrap();
                    }
                    writeln!(
                        out,
                        "\t\tif err := svc.{}(r.Context(), req, stream); err != nil {{",
                        method.method_name
                    )
                    .unwrap();
                    writeln!(out, "\t\t\txidlgohttp.WriteJSONError(w, http.StatusInternalServerError, \"INTERNAL\", err.Error())").unwrap();
                    writeln!(out, "\t\t\treturn").unwrap();
                    writeln!(out, "\t\t}}").unwrap();
                    writeln!(out, "\t\t_ = stream.Close()").unwrap();
                }
                Some(HttpStreamKind::Client) => {
                    writeln!(
                        out,
                        "\t\tstream := xidlgohttp.NewClientStreamReader[{}](r.Context(), r.Body)",
                        method.request_struct
                    )
                    .unwrap();
                    writeln!(
                        out,
                        "\t\tresp, err := svc.{}(r.Context(), stream)",
                        method.method_name
                    )
                    .unwrap();
                    writeln!(out, "\t\tif err != nil {{").unwrap();
                    writeln!(out, "\t\t\txidlgohttp.WriteJSONError(w, http.StatusInternalServerError, \"INTERNAL\", err.Error())").unwrap();
                    writeln!(out, "\t\t\treturn").unwrap();
                    writeln!(out, "\t\t}}").unwrap();
                    render_response_write(out, method, "resp", renderer)?;
                }
                Some(HttpStreamKind::Bidi) => {}
                None => {
                    if method.response_body_struct.is_some()
                        || method.response_body_direct_field.is_some()
                        || !method.response_header_params.is_empty()
                        || !method.response_cookie_params.is_empty()
                    {
                        writeln!(
                            out,
                            "\t\tresp, err := svc.{}(r.Context(), req)",
                            method.method_name
                        )
                        .unwrap();
                        writeln!(out, "\t\tif err != nil {{").unwrap();
                        writeln!(out, "\t\t\txidlgohttp.WriteJSONError(w, http.StatusInternalServerError, \"INTERNAL\", err.Error())").unwrap();
                        writeln!(out, "\t\t\treturn").unwrap();
                        writeln!(out, "\t\t}}").unwrap();
                        render_response_write(out, method, "resp", renderer)?;
                    } else {
                        writeln!(
                            out,
                            "\t\tif _, err := svc.{}(r.Context(), req); err != nil {{",
                            method.method_name
                        )
                        .unwrap();
                        writeln!(out, "\t\t\txidlgohttp.WriteJSONError(w, http.StatusInternalServerError, \"INTERNAL\", err.Error())").unwrap();
                        writeln!(out, "\t\t\treturn").unwrap();
                        writeln!(out, "\t\t}}").unwrap();
                        writeln!(out, "\t\tw.WriteHeader(http.StatusNoContent)").unwrap();
                    }
                }
            }
            writeln!(out, "\t}})").unwrap();
        }
    }
    writeln!(out, "\treturn mux").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn render_client(
    out: &mut String,
    interface_name: &str,
    methods: &[MethodMeta],
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    writeln!(out, "type {interface_name}Client struct {{").unwrap();
    writeln!(out, "\tbaseURL string").unwrap();
    writeln!(out, "\thttpClient *http.Client").unwrap();
    writeln!(out, "\tauth xidlgohttp.ClientAuth").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "func New{interface_name}Client(baseURL string, httpClient *http.Client, auth xidlgohttp.ClientAuth) *{interface_name}Client {{"
    )
    .unwrap();
    writeln!(out, "\tif httpClient == nil {{").unwrap();
    writeln!(out, "\t\thttpClient = http.DefaultClient").unwrap();
    writeln!(out, "\t}}").unwrap();
    writeln!(
        out,
        "\treturn &{interface_name}Client{{baseURL: strings.TrimRight(baseURL, \"/\"), httpClient: httpClient, auth: auth}}"
    )
    .unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    for method in methods {
        match method.stream_kind {
            Some(HttpStreamKind::Server) => {
                render_client_server_stream(out, interface_name, method, renderer)?
            }
            Some(HttpStreamKind::Client) => {
                render_client_client_stream(out, interface_name, method)?
            }
            Some(HttpStreamKind::Bidi) => {}
            None => render_client_unary(out, interface_name, method, renderer)?,
        }
    }
    Ok(())
}

fn render_client_unary(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    writeln!(
        out,
        "func (c *{interface_name}Client) {}(ctx context.Context, req *{}) (*{}, error) {{",
        method.method_name, method.request_struct, method.response_struct
    )
    .unwrap();
    render_client_build_request(out, method, renderer)?;
    writeln!(out, "\tresp, err := c.httpClient.Do(httpReq)").unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(out, "\tdefer resp.Body.Close()").unwrap();
    writeln!(
        out,
        "\tif resp.StatusCode >= 400 {{ return nil, fmt.Errorf(\"http %d\", resp.StatusCode) }}",
    )
    .unwrap();
    render_client_decode_response(out, method)?;
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn render_client_server_stream(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let item_ty = method
        .return_ty
        .clone()
        .unwrap_or_else(|| "string".to_string());
    let reader_ty = if method.stream_codec == HttpStreamCodec::Sse {
        format!("*xidlgohttp.SSEStreamReader[{item_ty}]")
    } else {
        format!("*xidlgohttp.ServerStreamReader[{item_ty}]")
    };
    writeln!(
        out,
        "func (c *{interface_name}Client) {}(ctx context.Context, req *{}) ({}, error) {{",
        method.method_name, method.request_struct, reader_ty
    )
    .unwrap();
    render_client_build_request(out, method, renderer)?;
    writeln!(out, "\tresp, err := c.httpClient.Do(httpReq)").unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(
        out,
        "\tif resp.StatusCode >= 400 {{ defer resp.Body.Close(); return nil, fmt.Errorf(\"http %d\", resp.StatusCode) }}",
    )
    .unwrap();
    if method.stream_codec == HttpStreamCodec::Sse {
        writeln!(
            out,
            "\treturn xidlgohttp.NewSSEStreamReader[{item_ty}](resp.Body), nil"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "\treturn xidlgohttp.NewNDJSONStreamReader[{item_ty}](resp.Body), nil"
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn render_client_client_stream(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
) -> IdlcResult<()> {
    writeln!(
        out,
        "func (c *{interface_name}Client) {}(ctx context.Context) (*xidlgohttp.ClientStreamWriter[{}, {}], error) {{",
        method.method_name, method.request_struct, method.response_struct
    )
    .unwrap();
    writeln!(
        out,
        "\trequestURL := c.baseURL + \"{}\"",
        method
            .paths
            .first()
            .cloned()
            .unwrap_or_else(|| "/".to_string())
    )
    .unwrap();
    writeln!(
        out,
        "\thttpReq, err := http.NewRequestWithContext(ctx, \"{}\", requestURL, nil)",
        super::definition::http_method_name(method.http_method)
    )
    .unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(
        out,
        "\thttpReq.Header.Set(\"Accept\", \"{}\")",
        method.response_content_type
    )
    .unwrap();
    writeln!(
        out,
        "\txidlgohttp.ApplyClientAuth(httpReq, c.auth, {}SecurityRequirements())",
        method.struct_prefix
    )
    .unwrap();
    writeln!(
        out,
        "\tstream := xidlgohttp.NewClientStreamWriter[{}, {}](ctx, c.httpClient, httpReq, func(resp *http.Response) ({}, error) {{",
        method.request_struct, method.response_struct, method.response_struct
    )
    .unwrap();
    writeln!(
        out,
        "\t\tif resp.StatusCode >= 400 {{ var zero {}; return zero, fmt.Errorf(\"http %d\", resp.StatusCode) }}",
        method.response_struct
    )
    .unwrap();
    writeln!(
        out,
        "\t\tdecoded, err := decode{}Response(resp)",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\t\treturn decoded, err").unwrap();
    writeln!(out, "\t}})").unwrap();
    writeln!(out, "\treturn stream, nil").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn render_client_build_request(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let mut query_encode = String::new();
    for param in &method.query_params {
        super::definition::emit_query_encode(&mut query_encode, param)?;
    }
    let mut header_encode = String::new();
    for param in &method.header_params {
        super::definition::emit_header_encode(&mut header_encode, param)?;
    }
    let mut cookie_encode = String::new();
    for param in &method.cookie_params {
        super::definition::emit_cookie_encode(&mut cookie_encode, param)?;
    }
    let ctx = ClientBuildRequestTemplate {
        method: ClientBuildRequestMethod {
            struct_prefix: &method.struct_prefix,
            http_method_name: super::definition::http_method_name(method.http_method),
            request_body_struct: method.request_body_struct.as_deref(),
            request_body_direct_field: method.request_body_direct_field.as_deref(),
            request_body_direct_ty: method.request_body_direct_ty.as_deref(),
            request_content_type: &method.request_content_type,
            response_content_type: &method.response_content_type,
            body_params: template_params(&method.body_params),
            has_query_params: !method.query_params.is_empty(),
            has_body_params: !method.body_params.is_empty(),
            has_security: !method.security.is_empty(),
            query_encode,
            header_encode,
            cookie_encode,
        },
    };
    out.push_str(&renderer.render_template("client_build_request.go.j2", &ctx)?);
    out.push('\n');
    Ok(())
}

fn render_client_decode_response(out: &mut String, method: &MethodMeta) -> IdlcResult<()> {
    writeln!(
        out,
        "\tdecoded, err := decode{}Response(resp)",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(out, "\treturn &decoded, nil").unwrap();
    Ok(())
}

fn render_decode_response_fn(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let mut response_header_decode = String::new();
    for param in &method.response_header_params {
        super::definition::emit_response_header_decode(&mut response_header_decode, param)?;
    }
    let mut response_cookie_decode = String::new();
    for param in &method.response_cookie_params {
        super::definition::emit_response_cookie_decode(&mut response_cookie_decode, param)?;
    }
    let ctx = DecodeResponseTemplate {
        method: DecodeResponseMethod {
            struct_prefix: &method.struct_prefix,
            response_struct: &method.response_struct,
            response_body_struct: method.response_body_struct.as_deref(),
            response_body_direct_field: method.response_body_direct_field.as_deref(),
            response_body_direct_ty: method.response_body_direct_ty.as_deref(),
            response_content_type: &method.response_content_type,
            return_ty: method.return_ty.as_deref(),
            response_body_params: template_params(&method.response_body_params),
            response_header_decode,
            response_cookie_decode,
        },
    };
    out.push_str(&renderer.render_template("decode_response.go.j2", &ctx)?);
    out.push('\n');
    Ok(())
}

fn render_request_binding(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let mut path_bindings = String::new();
    for param in &method.path_params {
        super::definition::emit_request_bind(&mut path_bindings, "r", param, "Path")?;
    }
    let mut query_bindings = String::new();
    for param in &method.query_params {
        super::definition::emit_request_bind(&mut query_bindings, "r.URL.Query()", param, "Query")?;
    }
    let mut header_bindings = String::new();
    for param in &method.header_params {
        super::definition::emit_request_bind(&mut header_bindings, "r.Header", param, "Header")?;
    }
    let mut cookie_bindings = String::new();
    for param in &method.cookie_params {
        super::definition::emit_request_bind(&mut cookie_bindings, "r", param, "Cookie")?;
    }
    let ctx = RequestBindingTemplate {
        method: RequestBindingMethod {
            is_client_stream: matches!(method.stream_kind, Some(HttpStreamKind::Client)),
            request_struct: &method.request_struct,
            request_body_struct: method.request_body_struct.as_deref(),
            request_body_direct_field: method.request_body_direct_field.as_deref(),
            request_body_direct_ty: method.request_body_direct_ty.as_deref(),
            request_content_type: &method.request_content_type,
            body_params: template_params(&method.body_params),
            path_bindings,
            query_bindings,
            header_bindings,
            cookie_bindings,
        },
    };
    out.push_str(&renderer.render_template("request_binding.go.j2", &ctx)?);
    out.push('\n');
    Ok(())
}

fn render_response_write(
    out: &mut String,
    method: &MethodMeta,
    value: &str,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let mut response_header_encode = String::new();
    for param in &method.response_header_params {
        super::definition::emit_response_header_encode(&mut response_header_encode, param, value)?;
    }
    let mut response_cookie_encode = String::new();
    for param in &method.response_cookie_params {
        super::definition::emit_response_cookie_encode(&mut response_cookie_encode, param, value)?;
    }
    let ctx = ResponseWriteTemplate {
        method: ResponseWriteMethod {
            response_body_struct: method.response_body_struct.as_deref(),
            response_body_direct_field: method.response_body_direct_field.as_deref(),
            response_body_direct_ty: method.response_body_direct_ty.as_deref(),
            response_content_type: &method.response_content_type,
            return_ty: method.return_ty.as_deref(),
            response_body_params: template_params(&method.response_body_params),
            response_header_encode,
            response_cookie_encode,
        },
        value,
    };
    out.push_str(&renderer.render_template("response_write.go.j2", &ctx)?);
    out.push('\n');
    Ok(())
}

fn render_method_types(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    writeln!(out, "type {} struct {{", method.request_struct).unwrap();
    for param in &method.request_params {
        writeln!(
            out,
            "\t{} {} `json:\"{}\" form:\"{}\"`",
            param.field_name, param.ty, param.raw_name, param.raw_name
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    if let Some(body_struct) = &method.request_body_struct {
        writeln!(out, "type {body_struct} struct {{").unwrap();
        for param in &method.body_params {
            writeln!(
                out,
                "\t{} {} `json:\"{}\" form:\"{}\"`",
                param.field_name, param.ty, param.raw_name, param.raw_name
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    writeln!(out, "type {} struct {{", method.response_struct).unwrap();
    if let Some(return_ty) = &method.return_ty {
        writeln!(out, "\tReturn {return_ty} `json:\"return\"`").unwrap();
    }
    for param in method
        .response_body_params
        .iter()
        .chain(method.response_header_params.iter())
        .chain(method.response_cookie_params.iter())
    {
        writeln!(
            out,
            "\t{} {} `json:\"{}\"`",
            param.field_name, param.ty, param.raw_name
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    if let Some(body_struct) = &method.response_body_struct {
        writeln!(out, "type {body_struct} struct {{").unwrap();
        if let Some(return_ty) = &method.return_ty {
            writeln!(out, "\tReturn {return_ty} `json:\"return\"`").unwrap();
        }
        for param in &method.response_body_params {
            writeln!(
                out,
                "\t{} {} `json:\"{}\"`",
                param.field_name, param.ty, param.raw_name
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    writeln!(
        out,
        "func {}SecurityRequirements() []xidlgohttp.SecurityRequirement {{",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\treturn []xidlgohttp.SecurityRequirement{{").unwrap();
    for requirement in &method.security {
        match requirement {
            HttpSecurityRequirement::HttpBasic => writeln!(
                out,
                "\t\t{{Kind: xidlgohttp.SecurityBasic, Realm: {:?}}},",
                method
                    .basic_realm
                    .clone()
                    .unwrap_or_else(|| method.method_name.clone())
            )
            .unwrap(),
            HttpSecurityRequirement::HttpBearer => {
                writeln!(out, "\t\t{{Kind: xidlgohttp.SecurityBearer}},").unwrap()
            }
            HttpSecurityRequirement::ApiKey { location, name } => {
                let loc = match location {
                    HttpApiKeyLocation::Header => "ApiKeyHeader",
                    HttpApiKeyLocation::Query => "ApiKeyQuery",
                    HttpApiKeyLocation::Cookie => "ApiKeyCookie",
                };
                writeln!(
                    out,
                    "\t\t{{Kind: xidlgohttp.SecurityAPIKey, Location: xidlgohttp.{loc}, Name: {:?}}},",
                    name
                )
                .unwrap();
            }
            HttpSecurityRequirement::OAuth2 { scopes } => {
                writeln!(
                    out,
                    "\t\t{{Kind: xidlgohttp.SecurityOAuth2, Scopes: []string{{{}}}}},",
                    scopes
                        .iter()
                        .map(|scope| format!("{scope:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .unwrap();
            }
        }
    }
    writeln!(out, "\t}}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "func {}Deprecated() xidlgohttp.DeprecatedInfo {{",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\treturn xidlgohttp.DeprecatedInfo{{").unwrap();
    writeln!(out, "\t\tDeprecated: {},", method.deprecated).unwrap();
    if let Some(since) = &method.deprecated_since {
        writeln!(out, "\t\tSince: {:?},", since).unwrap();
    }
    if let Some(after) = &method.deprecated_after {
        writeln!(out, "\t\tAfter: {:?},", after).unwrap();
    }
    if let Some(note) = &method.deprecated_note {
        writeln!(out, "\t\tNote: {:?},", note).unwrap();
    }
    writeln!(out, "\t}}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    super::definition::render_format_path_fn(out, method)?;
    writeln!(out).unwrap();
    render_decode_response_fn(out, method, renderer)?;
    Ok(())
}

fn template_params(params: &[super::ParamMeta]) -> Vec<MethodTemplateParam> {
    params
        .iter()
        .map(|param| MethodTemplateParam {
            field_name: param.field_name.clone(),
        })
        .collect()
}

pub(crate) fn build_method_meta(
    interface_name: &str,
    interface_annotations: &[hir::Annotation],
    op: &hir::OpDcl,
) -> IdlcResult<MethodMeta> {
    validate_http_annotations(&format!("operation '{}'", op.ident), &op.annotations)
        .map_err(IdlcError::rpc)?;
    let stream = http_stream_config(&op.annotations).map_err(IdlcError::rpc)?;
    validate_http_stream_target(
        &op.ident,
        stream,
        HttpStreamTargetSupport {
            target: "go-http",
            supports_bidi: false,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .map_err(IdlcError::rpc)?;
    let default_method = if matches!(stream.kind, Some(HttpStreamKind::Server)) {
        HttpMethod::Get
    } else {
        HttpMethod::Post
    };
    let (http_method, mut paths) =
        super::definition::route_from_annotations(&op.annotations, default_method)?;
    validate_http_stream_method(
        &op.ident,
        stream.kind,
        super::definition::http_method_name(http_method),
        HttpStreamTargetSupport {
            target: "go-http",
            supports_bidi: false,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .map_err(IdlcError::rpc)?;
    if paths.is_empty() {
        paths.push(super::definition::auto_default_method_path(
            op,
            http_method,
        )?);
    }
    let route_templates = paths
        .iter()
        .map(|path| super::definition::parse_route_template(path))
        .collect::<IdlcResult<Vec<_>>>()?;
    let all_path_param_names: HashSet<String> = route_templates
        .iter()
        .flat_map(|item| item.path_params.iter().cloned())
        .collect();
    let all_query_template_names: HashSet<String> = route_templates
        .iter()
        .flat_map(|item| item.query_params.iter().cloned())
        .collect();
    let default_source = super::definition::default_param_source(http_method);
    let deprecated = deprecated_context(interface_annotations, &op.annotations)?;

    let security_profile = effective_security_with_origin(interface_annotations, &op.annotations)
        .map_err(IdlcError::rpc)?;
    let (security, basic_realm) = match security_profile {
        None => (Vec::new(), None),
        Some(HttpSecurityProfile {
            origin: _,
            requirements,
        }) => (
            requirements,
            super::definition::find_basic_realm(&op.annotations)
                .or_else(|| super::definition::find_basic_realm(interface_annotations)),
        ),
    };

    let mut request_params = Vec::new();
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut header_params = Vec::new();
    let mut cookie_params = Vec::new();
    let mut body_params = Vec::new();
    let mut response_body_params = Vec::new();
    let mut response_header_params = Vec::new();
    let mut response_cookie_params = Vec::new();
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    for param in params {
        let direction = super::definition::param_direction(param.attr.as_ref());
        let meta = super::definition::param_meta(
            param,
            direction,
            default_source,
            &all_path_param_names,
            &all_query_template_names,
        )?;
        if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
            match meta.source {
                ParamSource::Header => response_header_params.push(meta.clone()),
                ParamSource::Cookie => response_cookie_params.push(meta.clone()),
                _ => response_body_params.push(meta.clone()),
            }
        }
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        request_params.push(meta.clone());
        match meta.source {
            ParamSource::Path => path_params.push(meta),
            ParamSource::Query => query_params.push(meta),
            ParamSource::Header => header_params.push(meta),
            ParamSource::Cookie => cookie_params.push(meta),
            ParamSource::Body => body_params.push(meta),
        }
    }

    if matches!(stream.kind, Some(HttpStreamKind::Client))
        && (!path_params.is_empty()
            || !query_params.is_empty()
            || !header_params.is_empty()
            || !cookie_params.is_empty())
    {
        return Err(IdlcError::rpc(format!(
            "@client_stream method '{}' currently supports body parameters only",
            op.ident
        )));
    }
    validate_query_template_bindings(&op.ident, &all_query_template_names, &query_params)?;

    let struct_prefix = format!("{}{}", interface_name, op.ident.to_case(Case::Pascal));
    let request_struct = format!("{struct_prefix}Request");
    let response_struct = format!("{struct_prefix}Response");
    let (request_body_struct, request_body_direct_field, request_body_direct_ty) =
        if body_params.is_empty() || matches!(stream.kind, Some(HttpStreamKind::Client)) {
            (None, None, None)
        } else if body_params.len() == 1 {
            let param = &body_params[0];
            (None, Some(param.field_name.clone()), Some(param.ty.clone()))
        } else {
            (Some(format!("{struct_prefix}RequestBody")), None, None)
        };
    let return_ty = match &op.ty {
        hir::OpTypeSpec::Void => None,
        hir::OpTypeSpec::TypeSpec(ty) => Some(super::definition::go_type(ty)),
    };
    let response_output_count = response_body_params.len() + usize::from(return_ty.is_some());
    let (response_body_struct, response_body_direct_field, response_body_direct_ty) =
        if response_output_count == 0 {
            (None, None, None)
        } else if response_output_count == 1 {
            if let Some(return_ty) = &return_ty {
                (None, Some("Return".to_string()), Some(return_ty.clone()))
            } else {
                let param = &response_body_params[0];
                (None, Some(param.field_name.clone()), Some(param.ty.clone()))
            }
        } else {
            (Some(format!("{struct_prefix}ResponseBody")), None, None)
        };
    Ok(MethodMeta {
        method_name: op.ident.to_case(Case::Pascal),
        struct_prefix,
        http_method,
        paths: route_templates
            .iter()
            .map(|item| item.path.clone())
            .collect(),
        request_struct,
        request_body_struct,
        request_body_direct_field,
        request_body_direct_ty,
        response_struct,
        response_body_struct,
        response_body_direct_field,
        response_body_direct_ty,
        request_content_type: effective_media_type(
            interface_annotations,
            &op.annotations,
            "Consumes",
        ),
        response_content_type: if matches!(stream.kind, Some(HttpStreamKind::Server))
            && stream.codec == HttpStreamCodec::Sse
        {
            "text/event-stream".to_string()
        } else if matches!(stream.kind, Some(HttpStreamKind::Client)) {
            "application/json".to_string()
        } else {
            effective_media_type(interface_annotations, &op.annotations, "Produces")
        },
        request_params,
        path_params,
        query_params,
        header_params,
        cookie_params,
        body_params,
        response_body_params,
        response_header_params,
        response_cookie_params,
        return_ty,
        stream_kind: stream.kind,
        stream_codec: stream.codec,
        security,
        basic_realm,
        deprecated: deprecated.deprecated,
        deprecated_since: deprecated.since,
        deprecated_after: deprecated.after,
        deprecated_note: deprecated.note,
    })
}

struct DeprecatedContext {
    deprecated: bool,
    since: Option<String>,
    after: Option<String>,
    note: Option<String>,
}

fn deprecated_context(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
) -> IdlcResult<DeprecatedContext> {
    let info = if let Some(info) = deprecated_info(method_annotations).map_err(IdlcError::rpc)? {
        Some(info)
    } else {
        deprecated_info(interface_annotations).map_err(IdlcError::rpc)?
    };
    let deprecated = info.as_ref().map(|value| value.deprecated).unwrap_or(false);
    let since = info.as_ref().and_then(|value| value.since.clone());
    let after = info.as_ref().and_then(|value| value.after.clone());
    let note = info.as_ref().map(|value| {
        let mut note = String::from("Deprecated.");
        if let Some(since) = &value.since {
            note.push_str(&format!(" Since {since}."));
        }
        if let Some(after) = &value.after {
            note.push_str(&format!(" After {after}."));
        }
        note
    });
    Ok(DeprecatedContext {
        deprecated,
        since,
        after,
        note,
    })
}

fn validate_query_template_bindings(
    method_name: &str,
    query_template_names: &HashSet<String>,
    query_params: &[super::ParamMeta],
) -> IdlcResult<()> {
    for name in query_template_names {
        let count = query_params
            .iter()
            .filter(|param| param.wire_name == *name)
            .count();
        if count == 0 {
            return Err(IdlcError::rpc(format!(
                "query template variable '{}' has no matching request-side query parameter in method '{}'",
                name, method_name
            )));
        }
        if count > 1 {
            return Err(IdlcError::rpc(format!(
                "query template variable '{}' is bound by multiple request-side query parameters in method '{}'",
                name, method_name
            )));
        }
    }
    Ok(())
}
