use crate::error::{IdlcError, IdlcResult};
use crate::generate::python_http::PythonHttpRenderer;
use crate::generate::utils::{
    HttpApiKeyLocation, HttpSecurityProfile, HttpSecurityRequirement, HttpStreamCodec,
    HttpStreamConfig, HttpStreamKind, HttpStreamTargetSupport, annotation_name, annotation_params,
    effective_media_type, effective_security_with_origin, has_optional_annotation,
    http_stream_config, normalize_annotation_params, validate_http_annotations,
    validate_http_stream_method, validate_http_stream_target,
};
use convert_case::{Case, Casing};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
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

struct SourceBinding {
    source: ParamSource,
    bound_name: String,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamDirection {
    In,
    Out,
    InOut,
}

pub(crate) fn render_spec(spec: &hir::Specification, module_name: &str) -> IdlcResult<String> {
    let renderer = PythonHttpRenderer::new()?;
    let mut body = String::new();
    for def in &spec.0 {
        render_definition(&mut body, def)?;
    }
    renderer.render_template(
        "spec.py.j2",
        &PythonHttpSpecTemplate {
            module_name: module_name.to_string(),
            body,
        },
    )
}

fn render_definition(out: &mut String, def: &hir::Definition) -> IdlcResult<()> {
    match def {
        hir::Definition::ModuleDcl(module) => {
            for inner in &module.definition {
                render_definition(out, inner)?;
            }
        }
        hir::Definition::InterfaceDcl(interface) => render_interface(out, interface)?,
        _ => {}
    }
    Ok(())
}

fn render_interface(out: &mut String, interface: &hir::InterfaceDcl) -> IdlcResult<()> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(());
    };
    validate_http_annotations(
        &format!("interface '{}'", def.header.ident),
        &interface.annotations,
    )
    .map_err(IdlcError::rpc)?;

    let interface_name = py_type_name(&def.header.ident);
    let mut methods = Vec::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            if let hir::Export::OpDcl(op) = export {
                methods.push(build_method(
                    interface.annotations.as_slice(),
                    op,
                    &interface_name,
                )?);
            }
        }
    }

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
        render_method_types(out, method, def)?;
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

fn build_method(
    interface_annotations: &[hir::Annotation],
    op: &hir::OpDcl,
    interface_name: &str,
) -> IdlcResult<MethodContext> {
    validate_http_annotations(&format!("operation '{}'", op.ident), &op.annotations)
        .map_err(IdlcError::rpc)?;
    let stream = http_stream_config(&op.annotations).map_err(IdlcError::rpc)?;
    let method = http_method(&op.annotations);
    validate_http_stream_target(
        &op.ident,
        stream,
        HttpStreamTargetSupport {
            target: "python-http",
            supports_bidi: false,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .map_err(IdlcError::rpc)?;
    validate_http_stream_method(
        &op.ident,
        stream.kind,
        &method,
        HttpStreamTargetSupport {
            target: "python-http",
            supports_bidi: false,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .map_err(IdlcError::rpc)?;

    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut request_params = Vec::new();
    let mut response_params = Vec::new();
    for param in params {
        let direction = param_direction(param.attr.as_ref());
        let flatten = has_flatten_annotation(&param.annotations);
        if flatten && matches!(direction, ParamDirection::Out) {
            return Err(IdlcError::rpc(format!(
                "@flatten can only be applied to request-side body parameter '{}' of method '{}'",
                param.declarator.0, op.ident
            )));
        }
        let binding = explicit_param_binding(param)?;
        let default_source = default_param_source(&method);
        let (source, wire_name) = match binding {
            Some(value) => (value.source, value.bound_name),
            None => (default_source, param.declarator.0.clone()),
        };
        if matches!(stream.kind, Some(HttpStreamKind::Client))
            && !matches!(direction, ParamDirection::Out)
            && !matches!(source, ParamSource::Body)
        {
            return Err(IdlcError::rpc(format!(
                "python-http @client_stream methods currently support body parameters only: '{}'",
                op.ident
            )));
        }
        let context = ParamContext {
            field_name: py_field_name(&param.declarator.0),
            wire_name,
            ty: py_type(&param.ty),
            optional: has_optional_annotation(&param.annotations),
            source,
            flatten,
        };
        if !matches!(direction, ParamDirection::Out) {
            request_params.push(context.clone());
        }
        if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
            response_params.push(context);
        }
    }

    let paths = collect_paths(op, &request_params, &method)?;
    let path_param_sets = paths
        .iter()
        .map(|path| parse_path_params(path))
        .collect::<Vec<_>>();
    for binding in &request_params {
        if matches!(binding.source, ParamSource::Path)
            && !path_param_sets
                .iter()
                .all(|set| set.contains(&binding.wire_name))
        {
            return Err(IdlcError::rpc(format!(
                "parameter '{}' is annotated with @path but '{}' is not present in every route template of method '{}'",
                binding.field_name, binding.wire_name, op.ident
            )));
        }
    }

    let request_type = format!("{}{}Request", interface_name, py_type_name(&op.ident));
    let response_type = match stream.kind {
        Some(HttpStreamKind::Server) => "ServerStreamResponse".to_string(),
        Some(HttpStreamKind::Client) | None => {
            format!("{}{}Response", interface_name, py_type_name(&op.ident))
        }
        Some(HttpStreamKind::Bidi) => "BidiStreamResponse".to_string(),
    };
    let body_param_count = request_params
        .iter()
        .filter(|value| matches!(value.source, ParamSource::Body))
        .count();
    for param in &request_params {
        if param.flatten && !matches!(param.source, ParamSource::Body) {
            return Err(IdlcError::rpc(format!(
                "@flatten can only be applied to body parameter '{}' of method '{}'",
                param.field_name, op.ident
            )));
        }
    }
    if body_param_count != 1 && request_params.iter().any(|value| value.flatten) {
        return Err(IdlcError::rpc(format!(
            "@flatten requires exactly one request-side body parameter, but method '{}' has {}",
            op.ident, body_param_count
        )));
    }
    let return_ty = match &op.ty {
        hir::OpTypeSpec::Void => None,
        hir::OpTypeSpec::TypeSpec(ty) => Some(py_type(ty)),
    };
    let request_content_type = request_content_type(interface_annotations, &op.annotations, stream);
    let response_content_type =
        response_content_type(interface_annotations, &op.annotations, stream);
    let requires_request_content_type = request_params
        .iter()
        .any(|value| matches!(value.source, ParamSource::Body))
        || matches!(stream.kind, Some(HttpStreamKind::Client));
    let security = effective_security_with_origin(interface_annotations, &op.annotations)
        .map_err(IdlcError::rpc)?;

    Ok(MethodContext {
        method_name: py_field_name(&op.ident),
        raw_name: op.ident.clone(),
        endpoint_name: format!(
            "_{}_{}_endpoint",
            py_field_name(interface_name),
            py_field_name(&op.ident)
        ),
        route_builder_name: format!(
            "_{}_{}_route",
            py_field_name(interface_name),
            py_field_name(&op.ident)
        ),
        http_method: method.clone(),
        paths,
        request_type,
        response_type,
        request_content_type,
        response_content_type,
        requires_request_content_type,
        security_expr: security_expr(security.as_ref()),
        stream_expr: stream_expr(stream),
        stream_kind: stream.kind,
        stream_codec: stream.codec,
        request_params,
        response_params,
        return_ty,
    })
}

fn render_method_types(
    out: &mut String,
    method: &MethodContext,
    def: &hir::InterfaceDef,
) -> IdlcResult<()> {
    let Some(_body) = &def.interface_body else {
        return Ok(());
    };
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

fn explicit_param_binding(param: &hir::ParamDcl) -> IdlcResult<Option<SourceBinding>> {
    let mut found = None;
    for annotation in &param.annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("path") {
            Some(ParamSource::Path)
        } else if name.eq_ignore_ascii_case("query") {
            Some(ParamSource::Query)
        } else if name.eq_ignore_ascii_case("body") {
            Some(ParamSource::Body)
        } else if name.eq_ignore_ascii_case("header") {
            Some(ParamSource::Header)
        } else if name.eq_ignore_ascii_case("cookie") {
            Some(ParamSource::Cookie)
        } else {
            None
        };
        let Some(current) = current else {
            continue;
        };
        let bound_name = annotation_params(annotation)
            .map(normalize_annotation_params)
            .and_then(|params| params.get("value").cloned())
            .unwrap_or_else(|| param.declarator.0.clone());
        if matches!(current, ParamSource::Header) {
            validate_header_name(&bound_name, &param.declarator.0)?;
        }
        if matches!(current, ParamSource::Cookie) {
            validate_cookie_name(&bound_name, &param.declarator.0)?;
        }
        match found {
            None => {
                found = Some(SourceBinding {
                    source: current,
                    bound_name,
                })
            }
            Some(ref previous)
                if previous.source == current && previous.bound_name == bound_name => {}
            Some(_) => {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' has conflicting source annotations (@path/@query/@body/@header/@cookie)",
                    param.declarator.0
                )));
            }
        }
    }
    Ok(found)
}

fn request_content_type(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
    stream: HttpStreamConfig,
) -> String {
    match stream.kind {
        Some(HttpStreamKind::Client) => "application/x-ndjson".to_string(),
        _ => effective_media_type(interface_annotations, method_annotations, "Consumes"),
    }
}

fn response_content_type(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
    stream: HttpStreamConfig,
) -> String {
    match (stream.kind, stream.codec) {
        (Some(HttpStreamKind::Server), HttpStreamCodec::Sse) => "text/event-stream".to_string(),
        _ => effective_media_type(interface_annotations, method_annotations, "Produces"),
    }
}

fn has_flatten_annotation(annotations: &[hir::Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("flatten"))
            .unwrap_or(false)
    })
}

fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

fn default_param_source(method: &str) -> ParamSource {
    match method {
        "GET" | "DELETE" | "HEAD" | "OPTIONS" => ParamSource::Query,
        _ => ParamSource::Body,
    }
}

fn collect_paths(
    op: &hir::OpDcl,
    params: &[ParamContext],
    method: &str,
) -> IdlcResult<Vec<String>> {
    let mut paths = Vec::new();
    for annotation in &op.annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if ["get", "post", "put", "patch", "delete", "head", "options"]
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
        {
            if let Some(params) = annotation_params(annotation) {
                let params = normalize_annotation_params(params);
                if let Some(path) = params.get("path") {
                    paths.push(normalize_path(path));
                }
            }
        } else if name.eq_ignore_ascii_case("path") {
            if let Some(params) = annotation_params(annotation) {
                let params = normalize_annotation_params(params);
                if let Some(path) = params.get("value") {
                    paths.push(normalize_path(path));
                }
            }
        }
    }
    if paths.is_empty() {
        let mut default_path = format!("/{}", op.ident);
        for param in params {
            if matches!(param.source, ParamSource::Path) {
                default_path.push('/');
                default_path.push('{');
                default_path.push_str(&param.wire_name);
                default_path.push('}');
            }
        }
        paths.push(normalize_path(&default_path));
    }
    if matches!(default_param_source(method), ParamSource::Query) {
        let query_names = params
            .iter()
            .filter(|param| matches!(param.source, ParamSource::Query))
            .map(|param| param.wire_name.clone())
            .collect::<Vec<_>>();
        for path in &paths {
            validate_route_template(path, &query_names)?;
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn validate_route_template(path: &str, query_names: &[String]) -> IdlcResult<()> {
    let mut start = 0usize;
    while let Some(open_rel) = path[start..].find('{') {
        let open = start + open_rel;
        let close = path[open + 1..]
            .find('}')
            .map(|value| open + 1 + value)
            .ok_or_else(|| {
                IdlcError::rpc(format!("route template has unmatched '{{' in '{path}'"))
            })?;
        let token = &path[open + 1..close];
        if token.starts_with('?') {
            for name in token[1..]
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                if !query_names.iter().any(|candidate| candidate == name) {
                    return Err(IdlcError::rpc(format!(
                        "query template variable '{}' has no matching request-side query parameter in route '{}'",
                        name, path
                    )));
                }
            }
        }
        start = close + 1;
    }
    Ok(())
}

fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut buf = String::new();
    let mut in_param = false;

    for ch in strip_query_template(path).chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
            }
            '}' if in_param => {
                if !buf.is_empty() && !buf.starts_with('?') {
                    out.insert(buf.trim_start_matches('*').to_string());
                }
                in_param = false;
            }
            _ => {
                if in_param {
                    buf.push(ch);
                }
            }
        }
    }

    out
}

fn strip_query_template(path: &str) -> &str {
    if let Some(pos) = path.find("{?") {
        &path[..pos]
    } else {
        path
    }
}

fn http_method(annotations: &[hir::Annotation]) -> String {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if ["get", "post", "put", "patch", "delete", "head", "options"]
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
        {
            return name.to_ascii_uppercase();
        }
    }
    match http_stream_config(annotations) {
        Ok(HttpStreamConfig {
            kind: Some(HttpStreamKind::Server),
            ..
        }) => "GET".to_string(),
        _ => "POST".to_string(),
    }
}

fn normalize_path(path: &str) -> String {
    let mut out = path.trim().replace("//", "/");
    if !out.starts_with('/') {
        out = format!("/{out}");
    }
    while out.len() > 1 && out.ends_with('/') {
        out.pop();
    }
    out
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

fn validate_header_name(bound_name: &str, param_name: &str) -> IdlcResult<()> {
    if bound_name.is_empty() {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has empty @header name",
            param_name
        )));
    }
    if bound_name.starts_with(':') {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' uses reserved pseudo-header name '{}'",
            param_name, bound_name
        )));
    }
    Ok(())
}

fn validate_cookie_name(bound_name: &str, param_name: &str) -> IdlcResult<()> {
    if bound_name.is_empty() {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has empty @cookie name",
            param_name
        )));
    }
    if bound_name
        .chars()
        .any(|ch| ch.is_ascii_whitespace() || ch == ';' || ch == '=')
    {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has invalid @cookie name '{}'",
            param_name, bound_name
        )));
    }
    Ok(())
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
                    format!(
                        "{}[{}]",
                        py_type_name(&value.ident),
                        value
                            .args
                            .iter()
                            .map(py_type)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        },
    }
}
