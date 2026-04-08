use crate::error::{IdlcError, IdlcResult};
use crate::generate::http_hir::{
    HttpHirDocument, HttpMethod, HttpOperation, HttpParam, HttpParamSource,
    semantics::{
        HttpApiKeyLocation, HttpSecurityProfile, HttpSecurityRequirement, HttpStreamCodec,
        HttpStreamConfig, HttpStreamKind,
    },
};
use crate::generate::python_http::PythonHttpRenderer;
use convert_case::{Case, Casing};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write;
use xidl_parser::hir;
use xidl_parser::hir::TypeSpec;

#[derive(Serialize)]
struct PythonHttpSpecTemplate {
    module_name: String,
    body: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamSource {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Clone)]
struct ParamContext {
    field_name: String,
    wire_name: String,
    ty: String,
    optional: bool,
    source: ParamSource,
    flatten: bool,
}

struct MethodContext {
    method_name: String,
    raw_name: String,
    endpoint_name: String,
    route_builder_name: String,
    http_method: String,
    paths: Vec<String>,
    request_type: String,
    response_type: String,
    request_content_type: String,
    response_content_type: String,
    requires_request_content_type: bool,
    security_expr: String,
    stream_expr: String,
    stream_kind: Option<HttpStreamKind>,
    stream_codec: HttpStreamCodec,
    request_params: Vec<ParamContext>,
    response_params: Vec<ParamContext>,
    return_ty: Option<String>,
}

pub(crate) fn render_spec(
    spec: &hir::Specification,
    module_name: &str,
    http_hir: &HttpHirDocument,
) -> IdlcResult<String> {
    let renderer = PythonHttpRenderer::new()?;
    let mut body = String::new();
    render_definitions(&mut body, &spec.0, &[], http_hir)?;
    renderer.render_template(
        "spec.py.j2",
        &PythonHttpSpecTemplate {
            module_name: module_name.to_string(),
            body,
        },
    )
}

fn render_definitions(
    out: &mut String,
    defs: &[hir::Definition],
    module_path: &[String],
    http_hir: &HttpHirDocument,
) -> IdlcResult<()> {
    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next = module_path.to_vec();
                next.push(module.ident.clone());
                render_definitions(out, &module.definition, &next, http_hir)?;
            }
            hir::Definition::InterfaceDcl(interface) => {
                render_interface(out, interface, module_path, http_hir)?
            }
            _ => {}
        }
    }
    Ok(())
}

fn render_interface(
    out: &mut String,
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    http_hir: &HttpHirDocument,
) -> IdlcResult<()> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(());
    };
    let Some(http_interface) = http_hir.find_interface(module_path, &def.header.ident) else {
        return Ok(());
    };

    let interface_name = py_type_name(&def.header.ident);
    let methods = http_interface
        .operations
        .iter()
        .filter(|operation| {
            !matches!(
                operation.source,
                crate::generate::http_hir::HttpOperationSource::AttributeGet
                    | crate::generate::http_hir::HttpOperationSource::AttributeSet
                    | crate::generate::http_hir::HttpOperationSource::AttributeWatch
            )
        })
        .map(|operation| build_method(operation, &interface_name))
        .collect::<IdlcResult<Vec<_>>>()?;

    let mut route_bindings = HashMap::<String, String>::new();
    for method in &methods {
        for path in &method.paths {
            let key = format!("{} {}", method.http_method, path);
            if let Some(previous) = route_bindings.insert(key.clone(), method.raw_name.clone()) {
                return Err(IdlcError::rpc(format!(
                    "duplicate HTTP route binding: {key} (methods: {previous}, {})",
                    method.raw_name
                )));
            }
        }
    }

    for method in &methods {
        render_method_types(out, method)?;
    }

    writeln!(out, "class {}Service(abc.ABC):", interface_name).unwrap();
    if methods.is_empty() {
        writeln!(out, "    pass").unwrap();
    } else {
        for method in &methods {
            writeln!(out, "    @abc.abstractmethod").unwrap();
            writeln!(
                out,
                "    async def {}(self, request: {}) -> {}:",
                method.method_name, method.request_type, method.response_type
            )
            .unwrap();
            writeln!(out, "        raise NotImplementedError").unwrap();
            writeln!(out).unwrap();
        }
    }

    for method in &methods {
        render_endpoint_helper(out, &interface_name, method)?;
        render_route_builder(out, &interface_name, method)?;
    }

    writeln!(
        out,
        "def {}_routes(service: {}Service) -> list[Route]:",
        py_field_name(&def.header.ident),
        interface_name
    )
    .unwrap();
    if methods.is_empty() {
        writeln!(out, "    return []").unwrap();
    } else {
        writeln!(out, "    return [").unwrap();
        for method in &methods {
            writeln!(out, "        {}(service),", method.route_builder_name).unwrap();
        }
        writeln!(out, "    ]").unwrap();
    }
    writeln!(out).unwrap();
    Ok(())
}

fn build_method(operation: &HttpOperation, interface_name: &str) -> IdlcResult<MethodContext> {
    let stream_kind = operation.stream.kind;
    let stream_codec = operation.stream.codec;
    if matches!(stream_kind, Some(HttpStreamKind::Bidi)) {
        return Err(IdlcError::rpc(format!(
            "python-http currently does not support @bidi_stream methods: '{}'",
            operation.name
        )));
    }
    if matches!(stream_kind, Some(HttpStreamKind::Server)) && stream_codec != HttpStreamCodec::Sse {
        return Err(IdlcError::rpc(format!(
            "python-http currently supports only SSE for @server_stream methods: '{}'",
            operation.name
        )));
    }
    if matches!(stream_kind, Some(HttpStreamKind::Client))
        && stream_codec != HttpStreamCodec::Ndjson
    {
        return Err(IdlcError::rpc(format!(
            "python-http currently supports only NDJSON for @client_stream methods: '{}'",
            operation.name
        )));
    }

    let request_params = operation
        .request_params
        .iter()
        .map(param_context)
        .collect::<Vec<_>>();
    let response_params = operation
        .response_params
        .iter()
        .map(param_context)
        .collect::<Vec<_>>();

    let request_type = format!("{}{}Request", interface_name, py_type_name(&operation.name));
    let response_type = match stream_kind {
        Some(HttpStreamKind::Server) => "ServerStreamResponse".to_string(),
        Some(HttpStreamKind::Client) | None => {
            format!(
                "{}{}Response",
                interface_name,
                py_type_name(&operation.name)
            )
        }
        Some(HttpStreamKind::Bidi) => unreachable!(),
    };
    let return_ty = operation.return_type.as_ref().map(py_type);
    let request_content_type = request_content_type(operation);
    let response_content_type = response_content_type(operation);
    let requires_request_content_type = !operation.request_body_params.is_empty()
        || matches!(stream_kind, Some(HttpStreamKind::Client));

    Ok(MethodContext {
        method_name: py_field_name(&operation.name),
        raw_name: operation.name.clone(),
        endpoint_name: format!(
            "_{}_{}_endpoint",
            py_field_name(interface_name),
            py_field_name(&operation.name)
        ),
        route_builder_name: format!(
            "_{}_{}_route",
            py_field_name(interface_name),
            py_field_name(&operation.name)
        ),
        http_method: http_method_name(operation.method).to_string(),
        paths: operation
            .routes
            .iter()
            .map(|route| route.path.clone())
            .collect(),
        request_type,
        response_type,
        request_content_type,
        response_content_type,
        requires_request_content_type,
        security_expr: security_expr(operation.security.as_ref()),
        stream_expr: stream_expr(operation.stream),
        stream_kind,
        stream_codec,
        request_params,
        response_params,
        return_ty,
    })
}

fn param_context(param: &HttpParam) -> ParamContext {
    ParamContext {
        field_name: py_field_name(&param.name),
        wire_name: param.wire_name.clone(),
        ty: py_type(&param.ty),
        optional: param.optional,
        source: param_source(param.source),
        flatten: param.flatten,
    }
}

fn render_method_types(out: &mut String, method: &MethodContext) -> IdlcResult<()> {
    writeln!(out, "@dataclass").unwrap();
    writeln!(out, "class {}:", method.request_type).unwrap();
    if method.request_params.is_empty() {
        writeln!(out, "    pass").unwrap();
    } else {
        for param in &method.request_params {
            writeln!(
                out,
                "    {}: {}",
                param.field_name,
                maybe_optional_type(param.optional, &param.ty)
            )
            .unwrap();
        }
    }
    writeln!(out).unwrap();

    if matches!(method.stream_kind, Some(HttpStreamKind::Server)) {
        return Ok(());
    }

    writeln!(out, "@dataclass").unwrap();
    writeln!(out, "class {}:", method.response_type).unwrap();
    let response_field_count =
        method.response_params.len() + usize::from(method.return_ty.is_some());
    if response_field_count == 0 {
        writeln!(out, "    pass").unwrap();
    } else if response_field_count == 1 && method.return_ty.is_some() {
        writeln!(
            out,
            "    value: {}",
            method.return_ty.as_deref().unwrap_or("Any")
        )
        .unwrap();
    } else {
        if let Some(return_ty) = &method.return_ty {
            writeln!(out, "    return_: {}", return_ty).unwrap();
        }
        for param in &method.response_params {
            writeln!(
                out,
                "    {}: {}",
                param.field_name,
                maybe_optional_type(param.optional, &param.ty)
            )
            .unwrap();
        }
    }
    writeln!(out).unwrap();
    Ok(())
}

fn render_endpoint_helper(
    out: &mut String,
    interface_name: &str,
    method: &MethodContext,
) -> IdlcResult<()> {
    writeln!(
        out,
        "async def {}(service: {}Service, request: Request):",
        method.endpoint_name, interface_name
    )
    .unwrap();
    writeln!(
        out,
        "    require_accept(request, {:?})",
        method.response_content_type
    )
    .unwrap();
    if method.requires_request_content_type {
        writeln!(
            out,
            "    require_content_type(request, {:?})",
            method.request_content_type
        )
        .unwrap();
    }
    if method.security_expr != "[]" {
        writeln!(
            out,
            "    _security = require_security(request, {})",
            method.security_expr
        )
        .unwrap();
    }
    if method
        .request_params
        .iter()
        .any(|param| matches!(param.source, ParamSource::Body))
    {
        writeln!(out, "    body = read_json_body(request)").unwrap();
    }
    if method.request_params.is_empty() {
        writeln!(out, "    request_value = {}()", method.request_type).unwrap();
    } else {
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
    writeln!(
        out,
        "    response_value = await service.{}(request_value)",
        method.method_name
    )
    .unwrap();
    match method.stream_kind {
        Some(HttpStreamKind::Server) => {
            writeln!(
                out,
                "    return encode_stream_response(response_value, codec={:?})",
                stream_codec_name(method.stream_codec)
            )
            .unwrap();
        }
        _ => {
            writeln!(out, "    return encode_json_response(response_value)").unwrap();
        }
    }
    writeln!(out).unwrap();
    Ok(())
}

fn render_route_builder(
    out: &mut String,
    interface_name: &str,
    method: &MethodContext,
) -> IdlcResult<()> {
    writeln!(
        out,
        "def {}(service: {}Service) -> Route:",
        method.route_builder_name, interface_name
    )
    .unwrap();
    writeln!(out, "    async def endpoint(request: Request):").unwrap();
    writeln!(
        out,
        "        return await {}(service, request)",
        method.endpoint_name
    )
    .unwrap();
    writeln!(out, "    return Route(").unwrap();
    writeln!(out, "        method={:?},", method.http_method).unwrap();
    writeln!(out, "        paths={:?},", method.paths).unwrap();
    writeln!(out, "        endpoint=endpoint,").unwrap();
    writeln!(out, "        handler=service.{},", method.method_name).unwrap();
    writeln!(out, "        request_model={},", method.request_type).unwrap();
    writeln!(out, "        response_model={},", method.response_type).unwrap();
    writeln!(
        out,
        "        metadata=RouteMetadata(request_content_type={:?}, response_content_type={:?}, security={}, stream={}),",
        method.request_content_type,
        method.response_content_type,
        method.security_expr,
        method.stream_expr
    )
    .unwrap();
    writeln!(out, "    )").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn render_param_binding(param: &ParamContext) -> String {
    match param.source {
        ParamSource::Path => format!(
            "read_scalar(path_value(request, {:?}), {:?}, optional={}, default_on_missing=False, wire_name={:?})",
            param.wire_name,
            param.ty,
            py_bool(param.optional),
            param.wire_name
        ),
        ParamSource::Query => format!(
            "read_scalar(query_value(request, {:?}), {:?}, optional={}, default_on_missing=False, wire_name={:?})",
            param.wire_name,
            param.ty,
            py_bool(param.optional),
            param.wire_name
        ),
        ParamSource::Header => format!(
            "read_scalar(header_value(request, {:?}), {:?}, optional={}, default_on_missing=False, wire_name={:?})",
            param.wire_name,
            param.ty,
            py_bool(param.optional),
            param.wire_name
        ),
        ParamSource::Cookie => format!(
            "read_scalar(cookie_value(request, {:?}), {:?}, optional={}, default_on_missing=False, wire_name={:?})",
            param.wire_name,
            param.ty,
            py_bool(param.optional),
            param.wire_name
        ),
        ParamSource::Body => {
            if param.flatten {
                format!(
                    "read_json_value(body, {:?}, optional={}, wire_name={:?})",
                    param.ty,
                    py_bool(param.optional),
                    param.wire_name
                )
            } else {
                format!(
                    "read_json_field(body, {:?}, {:?}, optional={})",
                    param.wire_name,
                    param.ty,
                    py_bool(param.optional)
                )
            }
        }
    }
}

fn request_content_type(operation: &HttpOperation) -> String {
    match operation.stream.kind {
        Some(HttpStreamKind::Client) => "application/x-ndjson".to_string(),
        _ => operation.request_content_type.clone(),
    }
}

fn response_content_type(operation: &HttpOperation) -> String {
    match (operation.stream.kind, operation.stream.codec) {
        (Some(HttpStreamKind::Server), HttpStreamCodec::Sse) => "text/event-stream".to_string(),
        _ => operation.response_content_type.clone(),
    }
}

fn security_expr(value: Option<&HttpSecurityProfile>) -> String {
    let Some(value) = value else {
        return "[]".to_string();
    };
    if value.requirements.is_empty() {
        return "[SecurityRequirement(kind=\"none\")]".to_string();
    }
    let parts = value
        .requirements
        .iter()
        .map(|requirement| match requirement {
            HttpSecurityRequirement::HttpBasic => "SecurityRequirement(kind=\"basic\")".to_string(),
            HttpSecurityRequirement::HttpBearer => {
                "SecurityRequirement(kind=\"bearer\")".to_string()
            }
            HttpSecurityRequirement::ApiKey { location, name } => format!(
                "SecurityRequirement(kind=\"api_key\", name={:?}, location={:?})",
                name,
                api_key_location(location)
            ),
            HttpSecurityRequirement::OAuth2 { scopes } => {
                format!("SecurityRequirement(kind=\"oauth2\", scopes={:?})", scopes)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{parts}]")
}

fn stream_expr(value: HttpStreamConfig) -> String {
    match value.kind {
        None => "None".to_string(),
        Some(kind) => format!(
            "StreamMetadata(kind={:?}, codec={:?})",
            match kind {
                HttpStreamKind::Server => "server",
                HttpStreamKind::Client => "client",
                HttpStreamKind::Bidi => "bidi",
            },
            stream_codec_name(value.codec)
        ),
    }
}

fn stream_codec_name(value: HttpStreamCodec) -> &'static str {
    match value {
        HttpStreamCodec::Sse => "sse",
        HttpStreamCodec::Ndjson => "ndjson",
    }
}

fn http_method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

fn param_source(source: HttpParamSource) -> ParamSource {
    match source {
        HttpParamSource::Path => ParamSource::Path,
        HttpParamSource::Query => ParamSource::Query,
        HttpParamSource::Header => ParamSource::Header,
        HttpParamSource::Cookie => ParamSource::Cookie,
        HttpParamSource::Body => ParamSource::Body,
    }
}

fn api_key_location(value: &HttpApiKeyLocation) -> &'static str {
    match value {
        HttpApiKeyLocation::Header => "header",
        HttpApiKeyLocation::Query => "query",
        HttpApiKeyLocation::Cookie => "cookie",
    }
}

fn py_type_name(value: &str) -> String {
    value.to_case(Case::Pascal)
}

fn py_field_name(value: &str) -> String {
    value.to_case(Case::Snake)
}

fn maybe_optional_type(optional: bool, ty: &str) -> String {
    if optional {
        format!("Optional[{ty}]")
    } else {
        ty.to_string()
    }
}

fn py_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

fn py_type(value: &TypeSpec) -> String {
    match value {
        TypeSpec::SimpleTypeSpec(value) => match value {
            hir::SimpleTypeSpec::IntegerType(_) => "int".to_string(),
            hir::SimpleTypeSpec::FloatingPtType => "float".to_string(),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => "str".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => value
                .name
                .iter()
                .map(|part| py_type_name(part))
                .collect::<Vec<_>>()
                .join("_"),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => "Any".to_string(),
        },
        TypeSpec::TemplateTypeSpec(value) => match value {
            hir::TemplateTypeSpec::SequenceType(value) => format!("list[{}]", py_type(&value.ty)),
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                "str".to_string()
            }
            hir::TemplateTypeSpec::FixedPtType(_) => "float".to_string(),
            hir::TemplateTypeSpec::MapType(value) => {
                format!("dict[{}, {}]", py_type(&value.key), py_type(&value.value))
            }
            hir::TemplateTypeSpec::TemplateType(value) => {
                if value.args.is_empty() {
                    py_type_name(&value.ident)
                } else {
                    "Any".to_string()
                }
            }
        },
    }
}
