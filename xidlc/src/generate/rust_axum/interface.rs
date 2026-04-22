use crate::error::{IdlcError, IdlcResult};
use crate::generate::http_hir::{
    HttpMethod as HttpHirMethod, HttpOperation, HttpOperationSource, HttpParam as HttpHirParam,
    HttpParamKind as HttpHirParamKind,
    semantics::{
        DeprecatedInfo, HttpApiKeyLocation, HttpSecurityOrigin, HttpSecurityProfile,
        HttpSecurityRequirement, HttpStreamCodec, HttpStreamKind, HttpStreamTargetSupport,
        deprecated_info, effective_media_type, effective_security_with_origin,
        has_optional_annotation, http_stream_config, validate_http_annotations,
        validate_http_stream_method, validate_http_stream_target,
    },
};
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_axum::transport::{TransportDirection, TransportTracker, TypeRegistry};
use crate::generate::rust_axum::{RustAxumRenderOutput, RustAxumRenderer};
use convert_case::{Case, Casing};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use xidl_parser::hir;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamSource {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamDirection {
    In,
    Out,
    InOut,
}

#[derive(Serialize)]
struct MethodContext {
    name: String,
    raw_name: String,
    rust_attrs: Vec<String>,
    deprecated: bool,
    deprecated_since: Option<String>,
    deprecated_after: Option<String>,
    deprecated_note: Option<String>,
    params: Vec<String>,
    param_names: Vec<String>,
    server_params: Vec<String>,
    server_param_names: Vec<String>,
    ret: String,
    response_ty: String,
    request_body_flatten: bool,
    http_method: String,
    http_method_fn: String,
    reqwest_method: String,
    path: String,
    paths: Vec<String>,
    struct_prefix: String,
    path_params: Vec<ParamContext>,
    query_params: Vec<ParamContext>,
    header_params: Vec<ParamContext>,
    cookie_params: Vec<ParamContext>,
    body_params: Vec<ParamContext>,
    request_ty: String,
    request_payload_ty: String,
    request_struct: Option<String>,
    auth_wrapper_struct: Option<String>,
    auth_in_request_struct: bool,
    has_basic_auth: bool,
    has_bearer_auth: bool,
    api_key_requirements: Vec<ApiKeyContext>,
    auth_source_interface: bool,
    auth_source_method: bool,
    auth_param: Option<String>,
    auth_param_ty: String,
    auth_ty: String,
    basic_auth_realm: String,
    request_params: Vec<ParamContext>,
    response_struct: Option<String>,
    response_params: Vec<ParamContext>,
    response_body_params: Vec<ParamContext>,
    response_header_params: Vec<ParamContext>,
    response_cookie_params: Vec<ParamContext>,
    response_include_return: bool,
    response_is_empty: bool,
    return_is_unit: bool,
    is_server_stream: bool,
    is_client_stream: bool,
    is_bidi_stream: bool,
    request_item_ty: String,
    ret_in_ty: String,
    ret_out_ty: String,
    request_content_type: String,
    response_content_type: String,
}

#[derive(Serialize, Clone)]
struct ParamContext {
    name: String,
    raw_name: String,
    wire_name: String,
    path_template_name: String,
    ty: String,
    in_ty: String,
    out_ty: String,
    source: String,
    serde_rename: Option<String>,
    header_is_multi: bool,
    header_item_ty: String,
    header_item_is_string: bool,
    header_item_is_primitive: bool,
    cookie_is_multi: bool,
    cookie_item_ty: String,
    cookie_item_is_string: bool,
    cookie_item_is_primitive: bool,
    optional: bool,
    inner_ty: String,
    flatten: bool,
}

#[derive(Serialize, Clone)]
struct ApiKeyContext {
    location: String,
    name: String,
}

#[allow(dead_code)]
#[derive(Serialize, Clone)]
struct RouteTemplate {
    path: String,
    path_params: HashSet<String>,
    query_params: HashSet<String>,
}

struct DeprecatedContext {
    deprecated: bool,
    since: Option<String>,
    after: Option<String>,
    note: Option<String>,
}

pub fn render_interface_with_path(
    interface: &hir::InterfaceDcl,
    renderer: &RustAxumRenderer,
    module_path: &[String],
    registry: &TypeRegistry,
) -> IdlcResult<RustAxumRenderOutput> {
    match &interface.decl {
        hir::InterfaceDclInner::InterfaceForwardDcl(_) => Ok(RustAxumRenderOutput::default()),
        hir::InterfaceDclInner::InterfaceDef(def) => {
            render_interface_def(def, &interface.annotations, renderer, module_path, registry)
        }
    }
}

fn render_interface_def(
    def: &hir::InterfaceDef,
    interface_annotations: &[hir::Annotation],
    renderer: &RustAxumRenderer,
    module_path: &[String],
    registry: &TypeRegistry,
) -> IdlcResult<RustAxumRenderOutput> {
    let mut out = RustAxumRenderOutput::default();
    let mut methods = Vec::new();
    let mut transport = TransportTracker::default();
    let http_hir = renderer.http_hir()?;
    let Some(http_interface) = http_hir.find_interface(module_path, &def.header.ident) else {
        return Ok(out);
    };

    if let Some(body) = &def.interface_body {
        for operation in &http_interface.operations {
            match operation.source {
                HttpOperationSource::Method => {
                    let op = body
                        .0
                        .iter()
                        .find_map(|export| match export {
                            hir::Export::OpDcl(op) if op.ident == operation.name => Some(op),
                            _ => None,
                        })
                        .ok_or_else(|| {
                            IdlcError::rpc(format!(
                                "missing source operation '{}' for rust-axum rendering",
                                operation.name
                            ))
                        })?;
                    methods.push(render_op_from_http(
                        op,
                        operation,
                        &def.header.ident,
                        registry,
                        &mut transport,
                    )?);
                }
                HttpOperationSource::AttributeGet
                | HttpOperationSource::AttributeSet
                | HttpOperationSource::AttributeWatch => {
                    let attr = body.0.iter().find_map(|export| match export {
                        hir::Export::AttrDcl(attr)
                            if attr_operation_names(attr).contains(&operation.name) =>
                        {
                            Some(attr)
                        }
                        _ => None,
                    });
                    methods.push(render_attr_operation_from_http(
                        attr,
                        operation,
                        &def.header.ident,
                        registry,
                        &mut transport,
                    )?);
                }
            }
        }
    }
    let transport_modules = transport.render_modules(registry, module_path)?;

    let mut route_bindings = HashMap::new();
    for method in &methods {
        for path in &method.paths {
            let key = format!("{} {}", method.reqwest_method, path);
            if let Some(previous) = route_bindings.insert(key.clone(), method.raw_name.clone()) {
                return Err(IdlcError::rpc(format!(
                    "duplicate HTTP route binding: {key} (methods: {previous}, {})",
                    method.raw_name
                )));
            }
        }
    }

    let ctx = serde_json::json!({
        "metadata": http_hir.document,
        "ident": rust_ident(&def.header.ident),
        "methods": methods,
        "rust_attrs": rust_passthrough_attrs_from_annotations(interface_annotations),
        "inbound_transport": transport_modules.inbound,
        "outbound_transport": transport_modules.outbound,
    });
    let rendered = renderer.render_template("interface.rs.j2", &ctx)?;
    out.source.push(rendered);
    Ok(out)
}

fn render_op_from_http(
    op: &hir::OpDcl,
    http_op: &HttpOperation,
    interface_name: &str,
    registry: &TypeRegistry,
    transport: &mut TransportTracker,
) -> IdlcResult<MethodContext> {
    let stream = http_op.stream;
    let is_server_stream = matches!(stream.kind, Some(HttpStreamKind::Server));
    let is_client_stream = matches!(stream.kind, Some(HttpStreamKind::Client));
    let is_bidi_stream = matches!(stream.kind, Some(HttpStreamKind::Bidi));
    let deprecated = deprecated_context_from_http(http_op);
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => axum_type(ty),
    };
    let ret_in_ty = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => {
            transport.map_type(ty, TransportDirection::In, registry)?
        }
    };
    let ret_out_ty = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => {
            transport.map_type(ty, TransportDirection::Out, registry)?
        }
    };
    let return_is_unit = matches!(&op.ty, hir::OpTypeSpec::Void);
    let (security, auth_source_interface, auth_source_method) = match &http_op.security {
        None => (None, false, false),
        Some(HttpSecurityProfile {
            origin,
            requirements,
        }) => (
            Some(requirements.clone()),
            matches!(origin, HttpSecurityOrigin::Interface),
            matches!(origin, HttpSecurityOrigin::Method),
        ),
    };
    let has_basic_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBasic))
        })
        .unwrap_or(false);
    let has_bearer_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBearer))
        })
        .unwrap_or(false);
    let api_key_requirements = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .filter_map(|req| match req {
                    HttpSecurityRequirement::ApiKey { location, name } => {
                        let location = match location {
                            HttpApiKeyLocation::Header => "Header",
                            HttpApiKeyLocation::Query => "Query",
                            HttpApiKeyLocation::Cookie => "Cookie",
                        };
                        Some(ApiKeyContext {
                            location: location.to_string(),
                            name: name.clone(),
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if has_basic_auth && has_bearer_auth {
        return Err(IdlcError::rpc(format!(
            "operation '{}' cannot combine @http_basic and @http_bearer",
            op.ident
        )));
    }
    let has_auth = has_basic_auth || has_bearer_auth;
    let auth_ty = if has_basic_auth {
        "xidl_rust_axum::auth::basic::BasicAuth".to_string()
    } else if has_bearer_auth {
        "xidl_rust_axum::auth::bearer::BearerAuth".to_string()
    } else {
        String::new()
    };
    let routes = &http_op.routes;
    let paths = routes
        .iter()
        .map(|route| route.path.clone())
        .collect::<Vec<_>>();
    let path = paths
        .first()
        .cloned()
        .unwrap_or_else(|| format!("/{}", op.ident));
    let _path_param_sets = routes
        .iter()
        .map(|route| route.path_params.iter().cloned().collect::<HashSet<_>>())
        .collect::<Vec<_>>();
    let _all_query_template_names = routes
        .iter()
        .flat_map(|route| route.query_params.iter().cloned())
        .collect::<HashSet<_>>();

    let mut param_list = Vec::new();
    let mut param_names = Vec::new();
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut header_params = Vec::new();
    let mut cookie_params = Vec::new();
    let mut body_params = Vec::new();
    let mut request_params = Vec::new();
    let mut response_params = Vec::new();
    let mut response_body_params = Vec::new();
    let mut response_header_params = Vec::new();
    let mut response_cookie_params = Vec::new();
    let mut path_binding_count = HashMap::<String, usize>::new();
    let mut query_binding_count = HashMap::<String, usize>::new();

    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);

    for param in params {
        let optional = has_optional_annotation(&param.annotations);
        let flatten = has_flatten_annotation(&param.annotations);
        let inner_ty = axum_type(&param.ty);
        let ty = render_param_type(&param.ty, param.attr.as_ref(), optional);
        let name = rust_ident(&param.declarator.0);
        let direction = param_direction(param.attr.as_ref());

        if matches!(direction, ParamDirection::Out | ParamDirection::InOut)
            && let Some(shared) = find_http_param(&http_op.response_params, &param.declarator.0)
        {
            let response_ctx = ParamContext {
                name: name.clone(),
                raw_name: param.declarator.0.clone(),
                wire_name: shared.wire_name.clone(),
                path_template_name: String::new(),
                ty: inner_ty.clone(),
                in_ty: transport_param_type(
                    &param.ty,
                    optional,
                    TransportDirection::In,
                    transport,
                    registry,
                )?,
                out_ty: transport_param_type(
                    &param.ty,
                    optional,
                    TransportDirection::Out,
                    transport,
                    registry,
                )?,
                source: param_source_code(http_param_kind(shared.kind)),
                serde_rename: field_rename(&param.annotations, &name)
                    .or_else(|| serde_rename(&param.declarator.0, &name)),
                header_is_multi: header_is_multi(&param.ty),
                header_item_ty: header_item_ty(&param.ty),
                header_item_is_string: header_item_is_string(&param.ty),
                header_item_is_primitive: header_item_is_primitive(&param.ty),
                cookie_is_multi: cookie_is_multi(&param.ty),
                cookie_item_ty: cookie_item_ty(&param.ty),
                cookie_item_is_string: cookie_item_is_string(&param.ty),
                cookie_item_is_primitive: cookie_item_is_primitive(&param.ty),
                optional,
                inner_ty: inner_ty.clone(),
                flatten: false,
            };
            match http_param_kind(shared.kind) {
                ParamSource::Header => response_header_params.push(response_ctx.clone()),
                ParamSource::Cookie => response_cookie_params.push(response_ctx.clone()),
                _ => response_body_params.push(response_ctx.clone()),
            }
            response_params.push(response_ctx);
        }
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        let Some(shared) = find_http_param(&http_op.request_params, &param.declarator.0) else {
            continue;
        };
        let source = http_param_kind(shared.kind);
        let wire_name = shared.wire_name.clone();
        param_list.push(format!("{name}: {ty}"));
        param_names.push(name.clone());
        let serde_name = if matches!(source, ParamSource::Body) {
            field_rename_raw(&param.annotations).unwrap_or_else(|| wire_name.clone())
        } else {
            wire_name.clone()
        };
        let ctx = ParamContext {
            name: name.clone(),
            raw_name: param.declarator.0.clone(),
            path_template_name: if matches!(source, ParamSource::Path)
                && path_param_is_catch_all(&path, &wire_name)
            {
                format!("*{wire_name}")
            } else {
                wire_name.clone()
            },
            wire_name,
            ty,
            in_ty: transport_param_type(
                &param.ty,
                optional,
                TransportDirection::In,
                transport,
                registry,
            )?,
            out_ty: transport_param_type(
                &param.ty,
                optional,
                TransportDirection::Out,
                transport,
                registry,
            )?,
            inner_ty: inner_ty.clone(),
            source: param_source_code(source),
            serde_rename: serde_rename(&serde_name, &name),
            header_is_multi: header_is_multi(&param.ty),
            header_item_ty: header_item_ty(&param.ty),
            header_item_is_string: header_item_is_string(&param.ty),
            header_item_is_primitive: header_item_is_primitive(&param.ty),
            cookie_is_multi: cookie_is_multi(&param.ty),
            cookie_item_ty: cookie_item_ty(&param.ty),
            cookie_item_is_string: cookie_item_is_string(&param.ty),
            cookie_item_is_primitive: cookie_item_is_primitive(&param.ty),
            optional,
            flatten,
        };
        request_params.push(ctx.clone());
        match source {
            ParamSource::Path => {
                *path_binding_count.entry(ctx.wire_name.clone()).or_insert(0) += 1;
                path_params.push(ctx);
            }
            ParamSource::Query => {
                *query_binding_count
                    .entry(ctx.wire_name.clone())
                    .or_insert(0) += 1;
                query_params.push(ctx);
            }
            ParamSource::Header => header_params.push(ctx),
            ParamSource::Cookie => cookie_params.push(ctx),
            ParamSource::Body => body_params.push(ctx),
        }
    }

    if is_client_stream
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
    if (is_client_stream || is_bidi_stream)
        && (!path_params.is_empty()
            || !query_params.is_empty()
            || !header_params.is_empty()
            || !cookie_params.is_empty())
    {
        return Err(IdlcError::rpc(format!(
            "@bidi_stream method '{}' currently supports body parameters only",
            op.ident
        )));
    }
    let method = http_method_from_hir(http_op.method);
    let method_name = rust_ident(&op.ident);
    let expose_auth_param = has_basic_auth || has_bearer_auth;
    let auth_in_request_struct = has_auth && !(is_client_stream || is_bidi_stream);
    let auth_wrapper_struct = if has_auth && (is_client_stream || is_bidi_stream) {
        Some(format!(
            "{}AuthRequest",
            method_struct_prefix(interface_name, &op.ident)
        ))
    } else {
        None
    };
    let mut request_struct = if request_params.is_empty() {
        None
    } else {
        Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &op.ident)
        ))
    };
    if auth_in_request_struct && request_struct.is_none() {
        request_struct = Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &op.ident)
        ));
    }
    let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
    let mut auth_param = None;
    let mut auth_param_ty = String::new();
    let mut server_params = param_list.clone();
    let mut server_param_names = param_names.clone();
    if auth_source_method && (has_basic_auth || has_bearer_auth) {
        let name = "xidl_auth".to_string();
        param_list.push(format!("{name}: {auth_ty}"));
        auth_param = Some(name);
        auth_param_ty = auth_ty.clone();
    }
    if !is_client_stream && !is_bidi_stream && expose_auth_param {
        let name = "xidl_auth".to_string();
        server_params.push(format!("{name}: {auth_ty}"));
        server_param_names.push(name);
    }
    let response_value_count = usize::from(!return_is_unit) + response_params.len();
    let response_body_count = usize::from(!return_is_unit) + response_body_params.len();
    let request_body_flatten = body_params.len() == 1 && body_params[0].flatten;
    let response_is_empty = response_body_count == 0;
    let response_include_return = !return_is_unit;
    let response_struct =
        if response_value_count > 1 || (return_is_unit && response_value_count == 1) {
            Some(format!(
                "{}Response",
                method_struct_prefix(interface_name, &op.ident)
            ))
        } else {
            None
        };
    let response_ty = if let Some(response_struct) = &response_struct {
        response_struct.clone()
    } else if !return_is_unit {
        ret.clone()
    } else if let Some(param) = response_params.first() {
        param.ty.clone()
    } else {
        "()".to_string()
    };
    let request_item_ty = request_ty.clone();
    let request_payload_ty = if is_client_stream {
        if let Some(wrapper) = &auth_wrapper_struct {
            wrapper.clone()
        } else {
            format!("xidl_rust_axum::stream::NdjsonStream<{}>", request_item_ty)
        }
    } else if is_bidi_stream {
        if let Some(wrapper) = &auth_wrapper_struct {
            wrapper.clone()
        } else {
            format!(
                "xidl_rust_axum::stream::BidiServerStream<{}, {}>",
                request_item_ty, response_ty
            )
        }
    } else {
        request_ty.clone()
    };
    let basic_auth_realm = if has_basic_auth {
        http_op
            .basic_auth_realm
            .clone()
            .unwrap_or_else(|| method_name.clone())
    } else {
        String::new()
    };
    Ok(MethodContext {
        name: method_name,
        raw_name: op.ident.clone(),
        rust_attrs: rust_passthrough_attrs_from_annotations(&op.annotations),
        deprecated: deprecated.deprecated,
        deprecated_since: deprecated.since,
        deprecated_after: deprecated.after,
        deprecated_note: deprecated.note,
        params: param_list,
        param_names,
        server_params,
        server_param_names,
        ret,
        response_ty,
        request_body_flatten,
        http_method: http_method_code(method),
        http_method_fn: http_method_fn(method),
        reqwest_method: reqwest_method_code(method),
        path,
        paths,
        struct_prefix: method_struct_prefix(interface_name, &op.ident),
        path_params,
        query_params,
        header_params,
        cookie_params,
        body_params,
        request_ty: request_ty.clone(),
        request_payload_ty,
        request_struct,
        auth_wrapper_struct,
        auth_in_request_struct,
        has_basic_auth,
        has_bearer_auth,
        api_key_requirements,
        auth_source_interface,
        auth_source_method,
        auth_param,
        auth_param_ty,
        auth_ty,
        basic_auth_realm,
        request_params,
        response_struct,
        response_params,
        response_body_params,
        response_header_params,
        response_cookie_params,
        response_include_return,
        response_is_empty,
        return_is_unit,
        is_server_stream,
        is_client_stream,
        is_bidi_stream,
        request_item_ty,
        ret_in_ty,
        ret_out_ty,
        request_content_type: if is_client_stream {
            "application/x-ndjson".to_string()
        } else {
            http_op.request_content_type.clone()
        },
        response_content_type: if is_server_stream {
            "text/event-stream".to_string()
        } else if is_client_stream {
            "application/json".to_string()
        } else {
            http_op.response_content_type.clone()
        },
    })
}

fn render_attr_operation_from_http(
    attr: Option<&hir::AttrDcl>,
    http_op: &HttpOperation,
    interface_name: &str,
    registry: &TypeRegistry,
    transport: &mut TransportTracker,
) -> IdlcResult<MethodContext> {
    let rust_attrs = attr
        .map(|attr| rust_passthrough_attrs_from_annotations(&attr.annotations))
        .unwrap_or_default();
    let deprecated = deprecated_context_from_http(http_op);
    let (security, auth_source_interface, auth_source_method) = match &http_op.security {
        None => (None, false, false),
        Some(HttpSecurityProfile {
            origin,
            requirements,
        }) => (
            Some(requirements.clone()),
            matches!(origin, HttpSecurityOrigin::Interface),
            matches!(origin, HttpSecurityOrigin::Method),
        ),
    };
    let has_basic_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBasic))
        })
        .unwrap_or(false);
    let has_bearer_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBearer))
        })
        .unwrap_or(false);
    let api_key_requirements = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .filter_map(|req| match req {
                    HttpSecurityRequirement::ApiKey { location, name } => {
                        let location = match location {
                            HttpApiKeyLocation::Header => "Header",
                            HttpApiKeyLocation::Query => "Query",
                            HttpApiKeyLocation::Cookie => "Cookie",
                        };
                        Some(ApiKeyContext {
                            location: location.to_string(),
                            name: name.clone(),
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let has_auth = has_basic_auth || has_bearer_auth;
    let auth_ty = if has_basic_auth {
        "xidl_rust_axum::auth::basic::BasicAuth".to_string()
    } else if has_bearer_auth {
        "xidl_rust_axum::auth::bearer::BearerAuth".to_string()
    } else {
        String::new()
    };
    let request_struct = if has_auth {
        Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &http_op.name)
        ))
    } else if !http_op.request_params.is_empty() {
        Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &http_op.name)
        ))
    } else {
        None
    };
    let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
    let mut params = Vec::new();
    let mut param_names = Vec::new();
    let mut server_params = Vec::new();
    let mut server_param_names = Vec::new();
    let mut request_params = Vec::new();
    let mut body_params = Vec::new();
    if let Some(param) = http_op
        .request_params
        .iter()
        .find(|param| matches!(param.kind, HttpHirParamKind::Body))
    {
        let param_name = rust_ident(&param.name);
        let ty = axum_type(&param.ty);
        params.push(format!("{param_name}: {ty}"));
        param_names.push(param_name.clone());
        server_params.push(format!("{param_name}: {ty}"));
        server_param_names.push(param_name.clone());
        let request_param = ParamContext {
            name: param_name,
            raw_name: param.name.clone(),
            wire_name: param.wire_name.clone(),
            path_template_name: String::new(),
            ty: ty.clone(),
            in_ty: transport.map_type(&param.ty, TransportDirection::In, registry)?,
            out_ty: transport.map_type(&param.ty, TransportDirection::Out, registry)?,
            source: param_source_code(ParamSource::Body),
            serde_rename: None,
            header_is_multi: false,
            header_item_ty: ty.clone(),
            header_item_is_string: false,
            header_item_is_primitive: false,
            cookie_is_multi: false,
            cookie_item_ty: ty.clone(),
            cookie_item_is_string: false,
            cookie_item_is_primitive: false,
            optional: false,
            inner_ty: ty.clone(),
            flatten: false,
        };
        request_params.push(request_param.clone());
        body_params.push(request_param);
    }
    let ret = http_op
        .return_type
        .as_ref()
        .map(axum_type)
        .unwrap_or_else(|| "()".to_string());
    let ret_in_ty = match &http_op.return_type {
        Some(ty) => transport.map_type(ty, TransportDirection::In, registry)?,
        None => "()".to_string(),
    };
    let ret_out_ty = match &http_op.return_type {
        Some(ty) => transport.map_type(ty, TransportDirection::Out, registry)?,
        None => "()".to_string(),
    };
    let return_is_unit = http_op.return_type.is_none();
    if has_basic_auth || has_bearer_auth {
        let name = "xidl_auth".to_string();
        server_params.push(format!("{name}: {auth_ty}"));
        server_param_names.push(name);
    }
    Ok(MethodContext {
        name: rust_ident(&http_op.name),
        raw_name: http_op.name.clone(),
        rust_attrs,
        deprecated: deprecated.deprecated,
        deprecated_since: deprecated.since,
        deprecated_after: deprecated.after,
        deprecated_note: deprecated.note,
        params,
        param_names,
        server_params,
        server_param_names,
        ret: ret.clone(),
        response_ty: ret.clone(),
        request_body_flatten: false,
        http_method: http_method_code(http_method_from_hir(http_op.method)),
        http_method_fn: http_method_fn(http_method_from_hir(http_op.method)),
        reqwest_method: reqwest_method_code(http_method_from_hir(http_op.method)),
        paths: http_op
            .routes
            .iter()
            .map(|route| route.path.clone())
            .collect(),
        path: http_op
            .routes
            .first()
            .map(|route| route.path.clone())
            .unwrap_or_default(),
        struct_prefix: method_struct_prefix(interface_name, &http_op.name),
        path_params: Vec::new(),
        query_params: Vec::new(),
        header_params: Vec::new(),
        cookie_params: Vec::new(),
        body_params,
        request_ty: request_ty.clone(),
        request_payload_ty: request_ty.clone(),
        request_struct,
        auth_wrapper_struct: None,
        auth_in_request_struct: has_auth,
        has_basic_auth,
        has_bearer_auth,
        api_key_requirements,
        auth_source_interface,
        auth_source_method,
        auth_param: None,
        auth_param_ty: String::new(),
        auth_ty,
        basic_auth_realm: http_op.basic_auth_realm.clone().unwrap_or_default(),
        request_params,
        response_struct: None,
        response_params: Vec::new(),
        response_body_params: Vec::new(),
        response_header_params: Vec::new(),
        response_cookie_params: Vec::new(),
        response_include_return: !return_is_unit,
        response_is_empty: return_is_unit,
        return_is_unit,
        is_server_stream: matches!(http_op.stream.kind, Some(HttpStreamKind::Server)),
        is_client_stream: matches!(http_op.stream.kind, Some(HttpStreamKind::Client)),
        is_bidi_stream: matches!(http_op.stream.kind, Some(HttpStreamKind::Bidi)),
        request_item_ty: "()".to_string(),
        ret_in_ty,
        ret_out_ty,
        request_content_type: if matches!(http_op.stream.kind, Some(HttpStreamKind::Client)) {
            "application/x-ndjson".to_string()
        } else {
            http_op.request_content_type.clone()
        },
        response_content_type: if matches!(http_op.stream.kind, Some(HttpStreamKind::Server)) {
            "text/event-stream".to_string()
        } else {
            http_op.response_content_type.clone()
        },
    })
}

fn find_http_param<'a>(params: &'a [HttpHirParam], name: &str) -> Option<&'a HttpHirParam> {
    params.iter().find(|param| param.name == name)
}

fn http_param_kind(source: HttpHirParamKind) -> ParamSource {
    match source {
        HttpHirParamKind::Path => ParamSource::Path,
        HttpHirParamKind::Query => ParamSource::Query,
        HttpHirParamKind::Header => ParamSource::Header,
        HttpHirParamKind::Cookie => ParamSource::Cookie,
        HttpHirParamKind::Body => ParamSource::Body,
    }
}

fn http_method_from_hir(method: HttpHirMethod) -> HttpMethod {
    match method {
        HttpHirMethod::Get => HttpMethod::Get,
        HttpHirMethod::Post => HttpMethod::Post,
        HttpHirMethod::Put => HttpMethod::Put,
        HttpHirMethod::Patch => HttpMethod::Patch,
        HttpHirMethod::Delete => HttpMethod::Delete,
        HttpHirMethod::Head => HttpMethod::Head,
        HttpHirMethod::Options => HttpMethod::Options,
    }
}

fn deprecated_context_from_http(http_op: &HttpOperation) -> DeprecatedContext {
    let info = http_op.deprecated.as_ref();
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
    DeprecatedContext {
        deprecated,
        since,
        after,
        note,
    }
}

fn attr_operation_names(attr: &hir::AttrDcl) -> Vec<String> {
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => match &spec.declarator {
            hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![
                format!("get_attribute_{}", decl.0),
                format!("watch_attribute_{}", decl.0),
            ],
            hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
        },
        hir::AttrDclInner::AttrSpec(spec) => match &spec.declarator {
            hir::AttrDeclarator::SimpleDeclarator(list) => list
                .iter()
                .flat_map(|decl| {
                    [
                        format!("get_attribute_{}", decl.0),
                        format!("set_attribute_{}", decl.0),
                        format!("watch_attribute_{}", decl.0),
                    ]
                })
                .collect(),
            hir::AttrDeclarator::WithRaises { declarator, .. } => vec![
                format!("get_attribute_{}", declarator.0),
                format!("set_attribute_{}", declarator.0),
                format!("watch_attribute_{}", declarator.0),
            ],
        },
    }
}

#[allow(dead_code)]
fn render_op(
    op: &hir::OpDcl,
    interface_annotations: &[hir::Annotation],
    interface_name: &str,
    _module_path: &[String],
    registry: &TypeRegistry,
    transport: &mut TransportTracker,
) -> IdlcResult<MethodContext> {
    validate_http_annotations(&format!("operation '{}'", op.ident), &op.annotations)
        .map_err(IdlcError::rpc)?;
    let stream = http_stream_config(&op.annotations).map_err(IdlcError::rpc)?;
    validate_http_stream_target(
        &op.ident,
        stream,
        HttpStreamTargetSupport {
            target: "rust-axum",
            supports_bidi: true,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .map_err(IdlcError::rpc)?;
    let is_server_stream = matches!(stream.kind, Some(HttpStreamKind::Server));
    let is_client_stream = matches!(stream.kind, Some(HttpStreamKind::Client));
    let is_bidi_stream = matches!(stream.kind, Some(HttpStreamKind::Bidi));
    let deprecated = deprecated_context(interface_annotations, &op.annotations)?;
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => axum_type(ty),
    };
    let ret_in_ty = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => {
            transport.map_type(ty, TransportDirection::In, registry)?
        }
    };
    let ret_out_ty = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => {
            transport.map_type(ty, TransportDirection::Out, registry)?
        }
    };
    let return_is_unit = matches!(&op.ty, hir::OpTypeSpec::Void);
    let security_profile = effective_security_with_origin(interface_annotations, &op.annotations)
        .map_err(IdlcError::rpc)?;
    let (security, auth_source_interface, auth_source_method) = match security_profile {
        None => (None, false, false),
        Some(HttpSecurityProfile {
            origin,
            requirements,
        }) => (
            Some(requirements),
            matches!(origin, HttpSecurityOrigin::Interface),
            matches!(origin, HttpSecurityOrigin::Method),
        ),
    };
    let has_basic_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBasic))
        })
        .unwrap_or(false);
    let has_bearer_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBearer))
        })
        .unwrap_or(false);
    let api_key_requirements = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .filter_map(|req| match req {
                    HttpSecurityRequirement::ApiKey { location, name } => {
                        let location = match location {
                            HttpApiKeyLocation::Header => "Header",
                            HttpApiKeyLocation::Query => "Query",
                            HttpApiKeyLocation::Cookie => "Cookie",
                        };
                        Some(ApiKeyContext {
                            location: location.to_string(),
                            name: name.clone(),
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if has_basic_auth && has_bearer_auth {
        return Err(IdlcError::rpc(format!(
            "operation '{}' cannot combine @http_basic and @http_bearer",
            op.ident
        )));
    }
    let has_auth = has_basic_auth || has_bearer_auth;
    let auth_ty = if has_basic_auth {
        "xidl_rust_axum::auth::basic::BasicAuth".to_string()
    } else if has_bearer_auth {
        "xidl_rust_axum::auth::bearer::BearerAuth".to_string()
    } else {
        String::new()
    };
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut param_list = Vec::new();
    let mut param_names = Vec::new();
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut header_params = Vec::new();
    let mut cookie_params = Vec::new();
    let mut body_params = Vec::new();
    let mut request_params = Vec::new();
    let mut response_params = Vec::new();
    let mut response_body_params = Vec::new();
    let mut response_header_params = Vec::new();
    let mut response_cookie_params = Vec::new();

    let default_method = if is_server_stream || is_bidi_stream {
        HttpMethod::Get
    } else {
        HttpMethod::Post
    };
    let (method, mut paths) = route_from_annotations(&op.annotations, default_method)?;
    validate_http_stream_method(
        &op.ident,
        stream.kind,
        method_name(method),
        HttpStreamTargetSupport {
            target: "rust-axum",
            supports_bidi: true,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .map_err(IdlcError::rpc)?;
    if paths.is_empty() {
        paths.push(auto_default_method_path(op, method)?);
    }
    validate_head_constraints(op, method)?;
    let route_templates = paths
        .iter()
        .map(|value| parse_route_template(value))
        .collect::<IdlcResult<Vec<_>>>()?;
    let paths = route_templates
        .iter()
        .map(|value| value.path.clone())
        .collect::<Vec<_>>();
    let path = paths
        .first()
        .cloned()
        .unwrap_or_else(|| format!("/{}", op.ident));
    let path_param_sets = route_templates
        .iter()
        .map(|value| value.path_params.clone())
        .collect::<Vec<_>>();
    let all_path_param_names: HashSet<String> = path_param_sets
        .iter()
        .flat_map(|set| set.iter().cloned())
        .collect();
    let all_query_template_names: HashSet<String> = route_templates
        .iter()
        .flat_map(|value| value.query_params.iter().cloned())
        .collect();
    let default_source = if is_bidi_stream {
        ParamSource::Body
    } else {
        default_param_source(method)
    };
    let mut path_binding_count = HashMap::<String, usize>::new();
    let mut query_binding_count = HashMap::<String, usize>::new();

    for param in params {
        let direction = param_direction(param.attr.as_ref());
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        if let Some(binding) = explicit_param_binding(param)? {
            if matches!(binding.source, ParamSource::Path)
                && !all_path_param_names.contains(&binding.bound_name)
            {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' is annotated with @path but '{}' is not present in any route template of method '{}'",
                    param.declarator.0, binding.bound_name, op.ident
                )));
            }
        }
    }

    for param in params {
        let optional = has_optional_annotation(&param.annotations);
        let flatten = has_flatten_annotation(&param.annotations);
        let inner_ty = axum_type(&param.ty);
        let ty = render_param_type(&param.ty, param.attr.as_ref(), optional);
        let name = rust_ident(&param.declarator.0);
        let direction = param_direction(param.attr.as_ref());
        if flatten && matches!(direction, ParamDirection::Out) {
            return Err(IdlcError::rpc(format!(
                "@flatten can only be applied to request-side body parameter '{}' of method '{}'",
                param.declarator.0, op.ident
            )));
        }
        if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
            let binding = explicit_param_binding(param)?;
            let (response_source, response_wire_name) = match binding {
                Some(binding)
                    if matches!(binding.source, ParamSource::Header | ParamSource::Cookie) =>
                {
                    (binding.source, binding.bound_name)
                }
                _ => (ParamSource::Body, param.declarator.0.clone()),
            };
            let response_ctx = ParamContext {
                name: name.clone(),
                raw_name: param.declarator.0.clone(),
                wire_name: response_wire_name,
                path_template_name: String::new(),
                ty: inner_ty.clone(),
                in_ty: transport_param_type(
                    &param.ty,
                    optional,
                    TransportDirection::In,
                    transport,
                    registry,
                )?,
                out_ty: transport_param_type(
                    &param.ty,
                    optional,
                    TransportDirection::Out,
                    transport,
                    registry,
                )?,
                source: param_source_code(response_source),
                serde_rename: field_rename(&param.annotations, &name)
                    .or_else(|| serde_rename(&param.declarator.0, &name)),
                header_is_multi: header_is_multi(&param.ty),
                header_item_ty: header_item_ty(&param.ty),
                header_item_is_string: header_item_is_string(&param.ty),
                header_item_is_primitive: header_item_is_primitive(&param.ty),
                cookie_is_multi: cookie_is_multi(&param.ty),
                cookie_item_ty: cookie_item_ty(&param.ty),
                cookie_item_is_string: cookie_item_is_string(&param.ty),
                cookie_item_is_primitive: cookie_item_is_primitive(&param.ty),
                optional,
                inner_ty: inner_ty.clone(),
                flatten: false,
            };
            if matches!(response_source, ParamSource::Header) {
                response_header_params.push(response_ctx.clone());
            }
            if matches!(response_source, ParamSource::Cookie) {
                response_cookie_params.push(response_ctx.clone());
            }
            if matches!(response_source, ParamSource::Body) {
                response_body_params.push(response_ctx.clone());
            }
            response_params.push(response_ctx);
        }
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        param_list.push(format!("{name}: {ty}"));
        param_names.push(name.clone());
        let binding = explicit_param_binding(param)?;
        let (source, wire_name) = match binding {
            Some(binding) => (binding.source, binding.bound_name),
            None if all_path_param_names.contains(&param.declarator.0) => {
                (ParamSource::Path, param.declarator.0.clone())
            }
            None if all_query_template_names.contains(&param.declarator.0) => {
                (ParamSource::Query, param.declarator.0.clone())
            }
            None => (default_source, param.declarator.0.clone()),
        };
        if matches!(source, ParamSource::Path)
            && !path_name_in_all_routes(&wire_name, &path_param_sets)
        {
            return Err(IdlcError::rpc(format!(
                "parameter '{}' is bound to path variable '{}' but it is not present in every route template of method '{}'",
                param.declarator.0, wire_name, op.ident
            )));
        }
        let serde_name = if matches!(source, ParamSource::Body) {
            field_rename_raw(&param.annotations).unwrap_or_else(|| wire_name.clone())
        } else {
            wire_name.clone()
        };
        let ctx = ParamContext {
            name: name.clone(),
            raw_name: param.declarator.0.clone(),
            path_template_name: if matches!(source, ParamSource::Path)
                && path_param_is_catch_all(&path, &wire_name)
            {
                format!("*{wire_name}")
            } else {
                wire_name.clone()
            },
            wire_name,
            ty,
            in_ty: transport_param_type(
                &param.ty,
                optional,
                TransportDirection::In,
                transport,
                registry,
            )?,
            out_ty: transport_param_type(
                &param.ty,
                optional,
                TransportDirection::Out,
                transport,
                registry,
            )?,
            inner_ty: inner_ty.clone(),
            source: param_source_code(source),
            serde_rename: serde_rename(&serde_name, &name),
            header_is_multi: header_is_multi(&param.ty),
            header_item_ty: header_item_ty(&param.ty),
            header_item_is_string: header_item_is_string(&param.ty),
            header_item_is_primitive: header_item_is_primitive(&param.ty),
            cookie_is_multi: cookie_is_multi(&param.ty),
            cookie_item_ty: cookie_item_ty(&param.ty),
            cookie_item_is_string: cookie_item_is_string(&param.ty),
            cookie_item_is_primitive: cookie_item_is_primitive(&param.ty),
            optional,
            flatten,
        };
        if ctx.optional && matches!(source, ParamSource::Path) {
            return Err(IdlcError::rpc(format!(
                "@optional cannot be applied to path parameter '{}' of method '{}'",
                param.declarator.0, op.ident
            )));
        }
        request_params.push(ctx.clone());
        match source {
            ParamSource::Path => {
                *path_binding_count.entry(ctx.wire_name.clone()).or_insert(0) += 1;
                path_params.push(ctx);
            }
            ParamSource::Query => {
                *query_binding_count
                    .entry(ctx.wire_name.clone())
                    .or_insert(0) += 1;
                query_params.push(ctx);
            }
            ParamSource::Header => header_params.push(ctx),
            ParamSource::Cookie => cookie_params.push(ctx),
            ParamSource::Body => body_params.push(ctx),
        }
    }
    let mut auth_param = None;
    let mut auth_param_ty = String::new();
    for route_params in &path_param_sets {
        for route_param in route_params {
            match path_binding_count.get(route_param).copied().unwrap_or(0) {
                0 => {
                    return Err(IdlcError::rpc(format!(
                        "route template variable '{}' has no matching request-side path parameter in method '{}'",
                        route_param, op.ident
                    )));
                }
                1 => {}
                _ => {
                    return Err(IdlcError::rpc(format!(
                        "route template variable '{}' is bound by multiple request-side path parameters in method '{}'",
                        route_param, op.ident
                    )));
                }
            }
        }
    }
    for route_template in &route_templates {
        for query_param in &route_template.query_params {
            match query_binding_count.get(query_param).copied().unwrap_or(0) {
                0 => {
                    return Err(IdlcError::rpc(format!(
                        "query template variable '{}' has no matching request-side query parameter in method '{}'",
                        query_param, op.ident
                    )));
                }
                1 => {}
                _ => {
                    return Err(IdlcError::rpc(format!(
                        "query template variable '{}' is bound by multiple request-side query parameters in method '{}'",
                        query_param, op.ident
                    )));
                }
            }
        }
    }

    let method_name = rust_ident(&op.ident);
    let auth_in_request_struct = has_auth && !(is_client_stream || is_bidi_stream);
    let auth_wrapper_struct = if has_auth && (is_client_stream || is_bidi_stream) {
        Some(format!(
            "{}AuthRequest",
            method_struct_prefix(interface_name, &op.ident)
        ))
    } else {
        None
    };
    let mut request_struct = if request_params.is_empty() {
        None
    } else {
        Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &op.ident)
        ))
    };
    if auth_in_request_struct && request_struct.is_none() {
        request_struct = Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &op.ident)
        ));
    }
    let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
    if is_client_stream && (!path_params.is_empty() || !query_params.is_empty()) {
        return Err(IdlcError::rpc(format!(
            "@client_stream method '{}' currently supports body parameters only",
            op.ident
        )));
    }
    if (is_client_stream || is_bidi_stream)
        && (!path_params.is_empty()
            || !query_params.is_empty()
            || !header_params.is_empty()
            || !cookie_params.is_empty())
    {
        return Err(IdlcError::rpc(format!(
            "@bidi_stream method '{}' currently supports body parameters only",
            op.ident
        )));
    }
    for param in &request_params {
        if param.flatten && param.source != param_source_code(ParamSource::Body) {
            return Err(IdlcError::rpc(format!(
                "@flatten can only be applied to body parameter '{}' of method '{}'",
                param.raw_name, op.ident
            )));
        }
    }
    if body_params.len() != 1 && body_params.iter().any(|param| param.flatten) {
        return Err(IdlcError::rpc(format!(
            "@flatten requires exactly one request-side body parameter, but method '{}' has {}",
            op.ident,
            body_params.len()
        )));
    }
    if is_client_stream || is_bidi_stream {
        param_list.clear();
        param_names.clear();
    }
    if auth_source_method && (has_basic_auth || has_bearer_auth) {
        let name = "xidl_auth".to_string();
        param_list.push(format!("{name}: {auth_ty}"));
        auth_param = Some(name);
        auth_param_ty = auth_ty.clone();
    }
    let mut server_params = param_list.clone();
    let mut server_param_names = param_names.clone();
    if !is_client_stream && !is_bidi_stream && (has_basic_auth || has_bearer_auth) {
        let name = "xidl_auth".to_string();
        server_params.push(format!("{name}: {auth_ty}"));
        server_param_names.push(name);
    }
    let response_value_count = usize::from(!return_is_unit) + response_params.len();
    let response_body_count = usize::from(!return_is_unit) + response_body_params.len();
    let request_body_flatten = body_params.len() == 1 && body_params[0].flatten;
    let response_is_empty = response_body_count == 0;
    let response_include_return = !return_is_unit;
    let response_struct =
        if response_value_count > 1 || (return_is_unit && response_value_count == 1) {
            Some(format!(
                "{}Response",
                method_struct_prefix(interface_name, &op.ident)
            ))
        } else {
            None
        };
    let response_ty = if let Some(response_struct) = &response_struct {
        response_struct.clone()
    } else if !return_is_unit {
        ret.clone()
    } else if let Some(param) = response_params.first() {
        param.ty.clone()
    } else {
        "()".to_string()
    };
    let request_item_ty = request_ty.clone();
    let request_payload_ty = if is_client_stream {
        if let Some(wrapper) = &auth_wrapper_struct {
            wrapper.clone()
        } else {
            format!("xidl_rust_axum::stream::NdjsonStream<{}>", request_item_ty)
        }
    } else if is_bidi_stream {
        if let Some(wrapper) = &auth_wrapper_struct {
            wrapper.clone()
        } else {
            format!(
                "xidl_rust_axum::stream::BidiServerStream<{}, {}>",
                request_item_ty, response_ty
            )
        }
    } else {
        request_ty.clone()
    };
    let basic_auth_realm = if has_basic_auth {
        find_basic_realm(&op.annotations)
            .or_else(|| find_basic_realm(interface_annotations))
            .unwrap_or_else(|| method_name.clone())
    } else {
        String::new()
    };
    Ok(MethodContext {
        name: method_name,
        raw_name: op.ident.clone(),
        rust_attrs: rust_passthrough_attrs_from_annotations(&op.annotations),
        deprecated: deprecated.deprecated,
        deprecated_since: deprecated.since,
        deprecated_after: deprecated.after,
        deprecated_note: deprecated.note,
        params: param_list,
        param_names,
        server_params,
        server_param_names,
        ret,
        response_ty,
        request_body_flatten,
        http_method: http_method_code(method),
        http_method_fn: http_method_fn(method),
        reqwest_method: reqwest_method_code(method),
        path,
        paths,
        struct_prefix: method_struct_prefix(interface_name, &op.ident),
        path_params,
        query_params,
        header_params,
        cookie_params,
        body_params,
        request_ty: request_ty.clone(),
        request_payload_ty,
        request_struct,
        auth_wrapper_struct,
        auth_in_request_struct,
        has_basic_auth,
        has_bearer_auth,
        api_key_requirements,
        auth_source_interface,
        auth_source_method,
        auth_param,
        auth_param_ty,
        auth_ty,
        basic_auth_realm,
        request_params,
        response_struct,
        response_params,
        response_body_params,
        response_header_params,
        response_cookie_params,
        response_include_return,
        response_is_empty,
        return_is_unit,
        is_server_stream,
        is_client_stream,
        is_bidi_stream,
        request_item_ty,
        ret_in_ty,
        ret_out_ty,
        request_content_type: effective_media_type(
            interface_annotations,
            &op.annotations,
            "Consumes",
        ),
        response_content_type: if is_server_stream {
            "text/event-stream".to_string()
        } else if is_client_stream {
            "application/json".to_string()
        } else {
            effective_media_type(interface_annotations, &op.annotations, "Produces")
        },
    })
}

#[allow(dead_code)]
fn render_attr(
    attr: &hir::AttrDcl,
    interface_annotations: &[hir::Annotation],
    interface_name: &str,
    module_path: &[String],
    _registry: &TypeRegistry,
    _transport: &mut TransportTracker,
) -> Vec<MethodContext> {
    let _ = validate_http_annotations(
        &format!("attribute in interface '{interface_name}'"),
        &attr.annotations,
    );
    let security_profile = effective_security_with_origin(interface_annotations, &attr.annotations)
        .unwrap_or_else(|err| panic!("{err}"));
    let (security, auth_source_interface, auth_source_method) = match security_profile {
        None => (None, false, false),
        Some(HttpSecurityProfile {
            origin,
            requirements,
        }) => (
            Some(requirements),
            matches!(origin, HttpSecurityOrigin::Interface),
            matches!(origin, HttpSecurityOrigin::Method),
        ),
    };
    let has_basic_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBasic))
        })
        .unwrap_or(false);
    let has_bearer_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBearer))
        })
        .unwrap_or(false);
    let api_key_requirements = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .filter_map(|req| match req {
                    HttpSecurityRequirement::ApiKey { location, name } => {
                        let location = match location {
                            HttpApiKeyLocation::Header => "Header",
                            HttpApiKeyLocation::Query => "Query",
                            HttpApiKeyLocation::Cookie => "Cookie",
                        };
                        Some(ApiKeyContext {
                            location: location.to_string(),
                            name: name.clone(),
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if has_basic_auth && has_bearer_auth {
        panic!(
            "attribute in interface '{interface_name}' cannot combine @http_basic and @http_bearer"
        );
    }
    let has_auth = has_basic_auth || has_bearer_auth;
    let auth_ty = if has_basic_auth {
        "xidl_rust_axum::auth::basic::BasicAuth".to_string()
    } else if has_bearer_auth {
        "xidl_rust_axum::auth::bearer::BearerAuth".to_string()
    } else {
        String::new()
    };
    let mut auth_param = None;
    let mut auth_param_ty = String::new();
    let mut auth_param_list = Vec::new();
    if auth_source_method && (has_basic_auth || has_bearer_auth) {
        let name = "xidl_auth".to_string();
        auth_param_list.push(format!("{name}: {auth_ty}"));
        auth_param = Some(name);
        auth_param_ty = auth_ty.clone();
    }
    let server_auth_param_list = if has_basic_auth || has_bearer_auth {
        vec![format!("xidl_auth: {auth_ty}")]
    } else {
        Vec::new()
    };
    let server_auth_param_names = if has_basic_auth || has_bearer_auth {
        vec!["xidl_auth".to_string()]
    } else {
        Vec::new()
    };
    let realm_hint = if has_basic_auth {
        find_basic_realm(&attr.annotations).or_else(|| find_basic_realm(interface_annotations))
    } else {
        None
    };
    let emit_watch = has_annotation(&attr.annotations, "server_stream");
    let rust_attrs = rust_passthrough_attrs_from_annotations(&attr.annotations);
    let deprecated = deprecated_context(interface_annotations, &attr.annotations)
        .unwrap_or_else(|err| panic!("{err}"));
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .flat_map(|names| {
                let ret = attr_return_type(&spec.ty);
                let raw = names.raw.clone();
                let getter_name = format!("get_attribute_{raw}");
                let path = attribute_path(&raw);
                let request_struct = if has_auth {
                    Some(format!(
                        "{}Request",
                        method_struct_prefix(interface_name, &raw)
                    ))
                } else {
                    None
                };
                let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
                let method_name = rust_ident(&getter_name);
                let basic_auth_realm = if has_basic_auth {
                    realm_hint.clone().unwrap_or_else(|| method_name.clone())
                } else {
                    String::new()
                };
                let mut methods = vec![MethodContext {
                    name: method_name,
                    raw_name: raw.clone(),
                    rust_attrs: rust_attrs.clone(),
                    deprecated: deprecated.deprecated,
                    deprecated_since: deprecated.since.clone(),
                    deprecated_after: deprecated.after.clone(),
                    deprecated_note: deprecated.note.clone(),
                    params: auth_param_list.clone(),
                    param_names: Vec::new(),
                    server_params: server_auth_param_list.clone(),
                    server_param_names: server_auth_param_names.clone(),
                    ret: ret.clone(),
                    response_ty: ret.clone(),
                    request_body_flatten: false,
                    http_method: http_method_code(HttpMethod::Get),
                    http_method_fn: http_method_fn(HttpMethod::Get),
                    reqwest_method: reqwest_method_code(HttpMethod::Get),
                    paths: vec![path.clone()],
                    path,
                    struct_prefix: method_struct_prefix(interface_name, &raw),
                    path_params: Vec::new(),
                    query_params: Vec::new(),
                    header_params: Vec::new(),
                    cookie_params: Vec::new(),
                    body_params: Vec::new(),
                    request_ty: request_ty.clone(),
                    request_payload_ty: request_ty.clone(),
                    request_struct,
                    auth_wrapper_struct: None,
                    auth_in_request_struct: has_auth,
                    has_basic_auth,
                    has_bearer_auth,
                    api_key_requirements: api_key_requirements.clone(),
                    auth_source_interface,
                    auth_source_method,
                    auth_param: auth_param.clone(),
                    auth_param_ty: auth_param_ty.clone(),
                    auth_ty: auth_ty.clone(),
                    basic_auth_realm,
                    request_params: Vec::new(),
                    response_struct: None,
                    response_params: Vec::new(),
                    response_body_params: Vec::new(),
                    response_header_params: Vec::new(),
                    response_cookie_params: Vec::new(),
                    response_include_return: true,
                    response_is_empty: false,
                    return_is_unit: false,
                    is_server_stream: false,
                    is_client_stream: false,
                    is_bidi_stream: false,
                    request_item_ty: "()".to_string(),
                    ret_in_ty: ret.clone(),
                    ret_out_ty: ret.clone(),
                    request_content_type: "application/json".to_string(),
                    response_content_type: "application/json".to_string(),
                }];
                if emit_watch {
                    let raw_watch = format!("watch_attribute_{raw}");
                    let watch_path = default_path(module_path, interface_name, &raw_watch);
                    let request_struct = if has_auth {
                        Some(format!(
                            "{}Request",
                            method_struct_prefix(interface_name, &raw_watch)
                        ))
                    } else {
                        None
                    };
                    let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
                    let method_name = rust_ident(&raw_watch);
                    let basic_auth_realm = if has_basic_auth {
                        realm_hint.clone().unwrap_or_else(|| method_name.clone())
                    } else {
                        String::new()
                    };
                    methods.push(MethodContext {
                        name: method_name,
                        raw_name: raw_watch.clone(),
                        rust_attrs: rust_attrs.clone(),
                        deprecated: deprecated.deprecated,
                        deprecated_since: deprecated.since.clone(),
                        deprecated_after: deprecated.after.clone(),
                        deprecated_note: deprecated.note.clone(),
                        params: auth_param_list.clone(),
                        param_names: Vec::new(),
                        server_params: Vec::new(),
                        server_param_names: Vec::new(),
                        ret: ret.clone(),
                        response_ty: ret.clone(),
                        request_body_flatten: false,
                        http_method: http_method_code(HttpMethod::Get),
                        http_method_fn: http_method_fn(HttpMethod::Get),
                        reqwest_method: reqwest_method_code(HttpMethod::Get),
                        paths: vec![watch_path.clone()],
                        path: watch_path,
                        struct_prefix: method_struct_prefix(interface_name, &raw_watch),
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        header_params: Vec::new(),
                        cookie_params: Vec::new(),
                        body_params: Vec::new(),
                        request_ty: request_ty.clone(),
                        request_payload_ty: request_ty,
                        request_struct,
                        auth_wrapper_struct: None,
                        auth_in_request_struct: has_auth,
                        has_basic_auth,
                        has_bearer_auth,
                        api_key_requirements: api_key_requirements.clone(),
                        auth_source_interface,
                        auth_source_method,
                        auth_param: auth_param.clone(),
                        auth_param_ty: auth_param_ty.clone(),
                        auth_ty: auth_ty.clone(),
                        basic_auth_realm,
                        request_params: Vec::new(),
                        response_struct: None,
                        response_params: Vec::new(),
                        response_body_params: Vec::new(),
                        response_header_params: Vec::new(),
                        response_cookie_params: Vec::new(),
                        response_include_return: true,
                        response_is_empty: false,
                        return_is_unit: false,
                        is_server_stream: true,
                        is_client_stream: false,
                        is_bidi_stream: false,
                        request_item_ty: "()".to_string(),
                        ret_in_ty: ret.clone(),
                        ret_out_ty: ret.clone(),
                        request_content_type: "application/json".to_string(),
                        response_content_type: "text/event-stream".to_string(),
                    });
                }
                methods
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let raw_name = decl.0.clone();
                        let getter_name = format!("get_attribute_{raw_name}");
                        let ret = attr_return_type(&spec.ty);
                        let path = attribute_path(&raw_name);
                        let request_struct = if has_auth {
                            Some(format!(
                                "{}Request",
                                method_struct_prefix(interface_name, &raw_name)
                            ))
                        } else {
                            None
                        };
                        let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
                        let method_name = rust_ident(&getter_name);
                        let basic_auth_realm = if has_basic_auth {
                            realm_hint.clone().unwrap_or_else(|| method_name.clone())
                        } else {
                            String::new()
                        };
                        out.push(MethodContext {
                            name: method_name,
                            raw_name: raw_name.clone(),
                            rust_attrs: rust_attrs.clone(),
                            deprecated: deprecated.deprecated,
                            deprecated_since: deprecated.since.clone(),
                            deprecated_after: deprecated.after.clone(),
                            deprecated_note: deprecated.note.clone(),
                            params: auth_param_list.clone(),
                            param_names: Vec::new(),
                            server_params: server_auth_param_list.clone(),
                            server_param_names: server_auth_param_names.clone(),
                            ret: ret.clone(),
                            response_ty: ret.clone(),
                            request_body_flatten: false,
                            http_method: http_method_code(HttpMethod::Get),
                            http_method_fn: http_method_fn(HttpMethod::Get),
                            reqwest_method: reqwest_method_code(HttpMethod::Get),
                            paths: vec![path.clone()],
                            path,
                            struct_prefix: method_struct_prefix(interface_name, &raw_name),
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            header_params: Vec::new(),
                            cookie_params: Vec::new(),
                            body_params: Vec::new(),
                            request_ty: request_ty.clone(),
                            request_payload_ty: request_ty,
                            request_struct,
                            auth_wrapper_struct: None,
                            auth_in_request_struct: has_auth,
                            has_basic_auth,
                            has_bearer_auth,
                            api_key_requirements: api_key_requirements.clone(),
                            auth_source_interface,
                            auth_source_method,
                            auth_param: auth_param.clone(),
                            auth_param_ty: auth_param_ty.clone(),
                            auth_ty: auth_ty.clone(),
                            basic_auth_realm,
                            request_params: Vec::new(),
                            response_struct: None,
                            response_params: Vec::new(),
                            response_body_params: Vec::new(),
                            response_header_params: Vec::new(),
                            response_cookie_params: Vec::new(),
                            response_include_return: true,
                            response_is_empty: false,
                            return_is_unit: false,
                            is_server_stream: false,
                            is_client_stream: false,
                            is_bidi_stream: false,
                            request_item_ty: "()".to_string(),
                            ret_in_ty: ret.clone(),
                            ret_out_ty: ret.clone(),
                            request_content_type: "application/json".to_string(),
                            response_content_type: "application/json".to_string(),
                        });
                        let raw_setter = format!("set_attribute_{raw_name}");
                        let setter = rust_ident(&raw_setter);
                        let param = render_param_type(&spec.ty, None, false);
                        let setter_path = attribute_path(&raw_name);
                        let request_struct = Some(format!(
                            "{}Request",
                            method_struct_prefix(interface_name, &raw_setter)
                        ));
                        let param_name = rust_ident(&raw_name);
                        let basic_auth_realm = if has_basic_auth {
                            realm_hint.clone().unwrap_or_else(|| setter.clone())
                        } else {
                            String::new()
                        };
                        let request_param = ParamContext {
                            name: param_name.clone(),
                            raw_name: raw_name.clone(),
                            wire_name: raw_name.clone(),
                            path_template_name: String::new(),
                            ty: param.clone(),
                            in_ty: param.clone(),
                            out_ty: param.clone(),
                            source: param_source_code(ParamSource::Body),
                            serde_rename: None,
                            header_is_multi: false,
                            header_item_ty: param.clone(),
                            header_item_is_string: false,
                            header_item_is_primitive: false,
                            cookie_is_multi: false,
                            cookie_item_ty: param.clone(),
                            cookie_item_is_string: false,
                            cookie_item_is_primitive: false,
                            optional: false,
                            inner_ty: param.clone(),
                            flatten: false,
                        };
                        let mut params = auth_param_list.clone();
                        params.push(format!("{param_name}: {param}"));
                        out.push(MethodContext {
                            name: setter.clone(),
                            raw_name: raw_setter.clone(),
                            rust_attrs: rust_attrs.clone(),
                            deprecated: deprecated.deprecated,
                            deprecated_since: deprecated.since.clone(),
                            deprecated_after: deprecated.after.clone(),
                            deprecated_note: deprecated.note.clone(),
                            params,
                            param_names: vec![param_name.clone()],
                            server_params: if has_auth {
                                vec![
                                    format!("{param_name}: {param}"),
                                    format!("xidl_auth: {auth_ty}"),
                                ]
                            } else {
                                vec![format!("{param_name}: {param}")]
                            },
                            server_param_names: if has_auth {
                                vec![param_name.clone(), "xidl_auth".to_string()]
                            } else {
                                vec![param_name.clone()]
                            },
                            ret: "()".to_string(),
                            response_ty: "()".to_string(),
                            request_body_flatten: false,
                            http_method: http_method_code(HttpMethod::Post),
                            http_method_fn: http_method_fn(HttpMethod::Post),
                            reqwest_method: reqwest_method_code(HttpMethod::Post),
                            paths: vec![setter_path.clone()],
                            path: setter_path,
                            struct_prefix: method_struct_prefix(interface_name, &raw_setter),
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            header_params: Vec::new(),
                            cookie_params: Vec::new(),
                            body_params: vec![request_param.clone()],
                            request_ty: request_struct.clone().unwrap_or_else(|| "()".to_string()),
                            request_payload_ty: request_struct
                                .clone()
                                .unwrap_or_else(|| "()".to_string()),
                            request_struct,
                            auth_wrapper_struct: None,
                            auth_in_request_struct: has_auth,
                            has_basic_auth,
                            has_bearer_auth,
                            api_key_requirements: api_key_requirements.clone(),
                            auth_source_interface,
                            auth_source_method,
                            auth_param: auth_param.clone(),
                            auth_param_ty: auth_param_ty.clone(),
                            auth_ty: auth_ty.clone(),
                            basic_auth_realm,
                            request_params: vec![request_param],
                            response_struct: None,
                            response_params: Vec::new(),
                            response_body_params: Vec::new(),
                            response_header_params: Vec::new(),
                            response_cookie_params: Vec::new(),
                            response_include_return: false,
                            response_is_empty: true,
                            return_is_unit: true,
                            is_server_stream: false,
                            is_client_stream: false,
                            is_bidi_stream: false,
                            request_item_ty: "()".to_string(),
                            ret_in_ty: "()".to_string(),
                            ret_out_ty: "()".to_string(),
                            request_content_type: "application/json".to_string(),
                            response_content_type: "application/json".to_string(),
                        });
                        if emit_watch {
                            let raw_watch = format!("watch_attribute_{raw_name}");
                            let watch_path = default_path(module_path, interface_name, &raw_watch);
                            let request_struct = if has_auth {
                                Some(format!(
                                    "{}Request",
                                    method_struct_prefix(interface_name, &raw_watch)
                                ))
                            } else {
                                None
                            };
                            let request_ty =
                                request_struct.clone().unwrap_or_else(|| "()".to_string());
                            let method_name = rust_ident(&raw_watch);
                            let basic_auth_realm = if has_basic_auth {
                                realm_hint.clone().unwrap_or_else(|| method_name.clone())
                            } else {
                                String::new()
                            };
                            out.push(MethodContext {
                                name: method_name,
                                raw_name: raw_watch.clone(),
                                rust_attrs: rust_attrs.clone(),
                                deprecated: deprecated.deprecated,
                                deprecated_since: deprecated.since.clone(),
                                deprecated_after: deprecated.after.clone(),
                                deprecated_note: deprecated.note.clone(),
                                params: auth_param_list.clone(),
                                param_names: Vec::new(),
                                server_params: Vec::new(),
                                server_param_names: Vec::new(),
                                ret: ret.clone(),
                                response_ty: ret.clone(),
                                request_body_flatten: false,
                                http_method: http_method_code(HttpMethod::Get),
                                http_method_fn: http_method_fn(HttpMethod::Get),
                                reqwest_method: reqwest_method_code(HttpMethod::Get),
                                paths: vec![watch_path.clone()],
                                path: watch_path,
                                struct_prefix: method_struct_prefix(interface_name, &raw_watch),
                                path_params: Vec::new(),
                                query_params: Vec::new(),
                                header_params: Vec::new(),
                                cookie_params: Vec::new(),
                                body_params: Vec::new(),
                                request_ty: request_ty.clone(),
                                request_payload_ty: request_ty,
                                request_struct,
                                auth_wrapper_struct: None,
                                auth_in_request_struct: has_auth,
                                has_basic_auth,
                                has_bearer_auth,
                                api_key_requirements: api_key_requirements.clone(),
                                auth_source_interface,
                                auth_source_method,
                                auth_param: auth_param.clone(),
                                auth_param_ty: auth_param_ty.clone(),
                                auth_ty: auth_ty.clone(),
                                basic_auth_realm,
                                request_params: Vec::new(),
                                response_struct: None,
                                response_params: Vec::new(),
                                response_body_params: Vec::new(),
                                response_header_params: Vec::new(),
                                response_cookie_params: Vec::new(),
                                response_include_return: true,
                                response_is_empty: false,
                                return_is_unit: false,
                                is_server_stream: true,
                                is_client_stream: false,
                                is_bidi_stream: false,
                                request_item_ty: "()".to_string(),
                                ret_in_ty: ret.clone(),
                                ret_out_ty: ret.clone(),
                                request_content_type: "application/json".to_string(),
                                response_content_type: "text/event-stream".to_string(),
                            });
                        }
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let raw_name = declarator.0.clone();
                    let getter_name = format!("get_attribute_{raw_name}");
                    let ret = attr_return_type(&spec.ty);
                    let path = attribute_path(&raw_name);
                    let param = render_param_type(&spec.ty, None, false);
                    let request_struct = if has_auth {
                        Some(format!(
                            "{}Request",
                            method_struct_prefix(interface_name, &raw_name)
                        ))
                    } else {
                        None
                    };
                    let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
                    let method_name = rust_ident(&getter_name);
                    let basic_auth_realm = if has_basic_auth {
                        realm_hint.clone().unwrap_or_else(|| method_name.clone())
                    } else {
                        String::new()
                    };
                    out.push(MethodContext {
                        name: method_name,
                        raw_name: raw_name.clone(),
                        rust_attrs: rust_attrs.clone(),
                        deprecated: deprecated.deprecated,
                        deprecated_since: deprecated.since.clone(),
                        deprecated_after: deprecated.after.clone(),
                        deprecated_note: deprecated.note.clone(),
                        params: auth_param_list.clone(),
                        param_names: Vec::new(),
                        server_params: server_auth_param_list.clone(),
                        server_param_names: server_auth_param_names.clone(),
                        ret: ret.clone(),
                        response_ty: ret.clone(),
                        request_body_flatten: false,
                        http_method: http_method_code(HttpMethod::Get),
                        http_method_fn: http_method_fn(HttpMethod::Get),
                        reqwest_method: reqwest_method_code(HttpMethod::Get),
                        paths: vec![path.clone()],
                        path,
                        struct_prefix: method_struct_prefix(interface_name, &raw_name),
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        header_params: Vec::new(),
                        cookie_params: Vec::new(),
                        body_params: Vec::new(),
                        request_ty: request_ty.clone(),
                        request_payload_ty: request_ty,
                        request_struct,
                        auth_wrapper_struct: None,
                        auth_in_request_struct: has_auth,
                        has_basic_auth,
                        has_bearer_auth,
                        api_key_requirements: api_key_requirements.clone(),
                        auth_source_interface,
                        auth_source_method,
                        auth_param: auth_param.clone(),
                        auth_param_ty: auth_param_ty.clone(),
                        auth_ty: auth_ty.clone(),
                        basic_auth_realm,
                        request_params: Vec::new(),
                        response_struct: None,
                        response_params: Vec::new(),
                        response_body_params: Vec::new(),
                        response_header_params: Vec::new(),
                        response_cookie_params: Vec::new(),
                        response_include_return: true,
                        response_is_empty: false,
                        return_is_unit: false,
                        is_server_stream: false,
                        is_client_stream: false,
                        is_bidi_stream: false,
                        request_item_ty: "()".to_string(),
                        ret_in_ty: ret.clone(),
                        ret_out_ty: ret.clone(),
                        request_content_type: "application/json".to_string(),
                        response_content_type: "application/json".to_string(),
                    });
                    let raw_setter = format!("set_attribute_{raw_name}");
                    let setter = rust_ident(&raw_setter);
                    let setter_path = attribute_path(&raw_name);
                    let request_struct = Some(format!(
                        "{}Request",
                        method_struct_prefix(interface_name, &raw_setter)
                    ));
                    let param_name = rust_ident(&raw_name);
                    let basic_auth_realm = if has_basic_auth {
                        realm_hint.clone().unwrap_or_else(|| setter.clone())
                    } else {
                        String::new()
                    };
                    let request_param = ParamContext {
                        name: param_name.clone(),
                        raw_name: raw_name.clone(),
                        wire_name: raw_name.clone(),
                        path_template_name: String::new(),
                        ty: param.clone(),
                        in_ty: param.clone(),
                        out_ty: param.clone(),
                        source: param_source_code(ParamSource::Body),
                        serde_rename: None,
                        header_is_multi: false,
                        header_item_ty: param.clone(),
                        header_item_is_string: false,
                        header_item_is_primitive: false,
                        cookie_is_multi: false,
                        cookie_item_ty: param.clone(),
                        cookie_item_is_string: false,
                        cookie_item_is_primitive: false,
                        optional: false,
                        inner_ty: param.clone(),
                        flatten: false,
                    };
                    let mut params = auth_param_list.clone();
                    params.push(format!("{param_name}: {param}"));
                    out.push(MethodContext {
                        name: setter.clone(),
                        raw_name: raw_setter.clone(),
                        rust_attrs: rust_attrs.clone(),
                        deprecated: deprecated.deprecated,
                        deprecated_since: deprecated.since.clone(),
                        deprecated_after: deprecated.after.clone(),
                        deprecated_note: deprecated.note.clone(),
                        params,
                        param_names: vec![param_name.clone()],
                        server_params: if has_auth {
                            vec![
                                format!("{param_name}: {param}"),
                                format!("xidl_auth: {auth_ty}"),
                            ]
                        } else {
                            vec![format!("{param_name}: {param}")]
                        },
                        server_param_names: if has_auth {
                            vec![param_name.clone(), "xidl_auth".to_string()]
                        } else {
                            vec![param_name.clone()]
                        },
                        ret: "()".to_string(),
                        response_ty: "()".to_string(),
                        request_body_flatten: false,
                        http_method: http_method_code(HttpMethod::Post),
                        http_method_fn: http_method_fn(HttpMethod::Post),
                        reqwest_method: reqwest_method_code(HttpMethod::Post),
                        paths: vec![setter_path.clone()],
                        path: setter_path,
                        struct_prefix: method_struct_prefix(interface_name, &raw_setter),
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        header_params: Vec::new(),
                        cookie_params: Vec::new(),
                        body_params: vec![request_param.clone()],
                        request_ty: request_struct.clone().unwrap_or_else(|| "()".to_string()),
                        request_payload_ty: request_struct
                            .clone()
                            .unwrap_or_else(|| "()".to_string()),
                        request_struct,
                        auth_wrapper_struct: None,
                        auth_in_request_struct: has_auth,
                        has_basic_auth,
                        has_bearer_auth,
                        api_key_requirements: api_key_requirements.clone(),
                        auth_source_interface,
                        auth_source_method,
                        auth_param: auth_param.clone(),
                        auth_param_ty: auth_param_ty.clone(),
                        auth_ty: auth_ty.clone(),
                        basic_auth_realm,
                        request_params: vec![request_param],
                        response_struct: None,
                        response_params: Vec::new(),
                        response_body_params: Vec::new(),
                        response_header_params: Vec::new(),
                        response_cookie_params: Vec::new(),
                        response_include_return: false,
                        response_is_empty: true,
                        return_is_unit: true,
                        is_server_stream: false,
                        is_client_stream: false,
                        is_bidi_stream: false,
                        request_item_ty: "()".to_string(),
                        ret_in_ty: "()".to_string(),
                        ret_out_ty: "()".to_string(),
                        request_content_type: "application/json".to_string(),
                        response_content_type: "application/json".to_string(),
                    });
                    if emit_watch {
                        let raw_watch = format!("watch_attribute_{raw_name}");
                        let watch_path = default_path(module_path, interface_name, &raw_watch);
                        let request_struct = if has_auth {
                            Some(format!(
                                "{}Request",
                                method_struct_prefix(interface_name, &raw_watch)
                            ))
                        } else {
                            None
                        };
                        let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
                        let method_name = rust_ident(&raw_watch);
                        let basic_auth_realm = if has_basic_auth {
                            realm_hint.clone().unwrap_or_else(|| method_name.clone())
                        } else {
                            String::new()
                        };
                        out.push(MethodContext {
                            name: method_name,
                            raw_name: raw_watch.clone(),
                            rust_attrs: rust_attrs.clone(),
                            deprecated: deprecated.deprecated,
                            deprecated_since: deprecated.since.clone(),
                            deprecated_after: deprecated.after.clone(),
                            deprecated_note: deprecated.note.clone(),
                            params: auth_param_list.clone(),
                            param_names: Vec::new(),
                            server_params: Vec::new(),
                            server_param_names: Vec::new(),
                            ret: ret.clone(),
                            response_ty: ret.clone(),
                            request_body_flatten: false,
                            http_method: http_method_code(HttpMethod::Get),
                            http_method_fn: http_method_fn(HttpMethod::Get),
                            reqwest_method: reqwest_method_code(HttpMethod::Get),
                            paths: vec![watch_path.clone()],
                            path: watch_path,
                            struct_prefix: method_struct_prefix(interface_name, &raw_watch),
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            header_params: Vec::new(),
                            cookie_params: Vec::new(),
                            body_params: Vec::new(),
                            request_ty: request_ty.clone(),
                            request_payload_ty: request_ty,
                            request_struct,
                            auth_wrapper_struct: None,
                            auth_in_request_struct: has_auth,
                            has_basic_auth,
                            has_bearer_auth,
                            api_key_requirements: api_key_requirements.clone(),
                            auth_source_interface,
                            auth_source_method,
                            auth_param: auth_param.clone(),
                            auth_param_ty: auth_param_ty.clone(),
                            auth_ty: auth_ty.clone(),
                            basic_auth_realm,
                            request_params: Vec::new(),
                            response_struct: None,
                            response_params: Vec::new(),
                            response_body_params: Vec::new(),
                            response_header_params: Vec::new(),
                            response_cookie_params: Vec::new(),
                            response_include_return: true,
                            response_is_empty: false,
                            return_is_unit: false,
                            is_server_stream: true,
                            is_client_stream: false,
                            is_bidi_stream: false,
                            request_item_ty: "()".to_string(),
                            ret_in_ty: ret.clone(),
                            ret_out_ty: ret.clone(),
                            request_content_type: "application/json".to_string(),
                            response_content_type: "text/event-stream".to_string(),
                        });
                    }
                }
            }
            out
        }
    }
}

#[allow(dead_code)]
struct AttrNames {
    raw: String,
}

#[allow(dead_code)]
fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<AttrNames> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![AttrNames {
            raw: decl.0.clone(),
        }],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

#[allow(dead_code)]
fn attr_return_type(ty: &hir::TypeSpec) -> String {
    axum_type(ty)
}

#[allow(dead_code)]
fn effective_deprecated(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
) -> Result<Option<DeprecatedInfo>, String> {
    if let Some(info) = deprecated_info(method_annotations)? {
        return Ok(Some(info));
    }
    deprecated_info(interface_annotations)
}

#[allow(dead_code)]
fn deprecated_context(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
) -> IdlcResult<DeprecatedContext> {
    let info =
        effective_deprecated(interface_annotations, method_annotations).map_err(IdlcError::rpc)?;
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

#[allow(dead_code)]
fn find_basic_realm(annotations: &[hir::Annotation]) -> Option<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("http_basic") {
            continue;
        }
        let params = annotation_params(annotation)?;
        let params = normalize_params(params);
        if let Some(value) = params
            .get("realm")
            .cloned()
            .filter(|value| !value.is_empty())
        {
            return Some(value);
        }
        if let Some(value) = params
            .get("relm")
            .cloned()
            .filter(|value| !value.is_empty())
        {
            return Some(value);
        }
    }
    None
}

fn render_param_type(
    ty: &hir::TypeSpec,
    attr: Option<&hir::ParamAttribute>,
    optional: bool,
) -> String {
    let _ = attr;
    let inner = axum_type(ty);
    if optional {
        format!("Option<{inner}>")
    } else {
        inner
    }
}

fn transport_param_type(
    ty: &hir::TypeSpec,
    optional: bool,
    direction: TransportDirection,
    transport: &mut TransportTracker,
    registry: &TypeRegistry,
) -> IdlcResult<String> {
    let inner = transport.map_type(ty, direction, registry)?;
    Ok(if optional {
        format!("Option<{inner}>")
    } else {
        inner
    })
}

fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

fn method_struct_prefix(interface_name: &str, method_name: &str) -> String {
    let interface = interface_name.strip_prefix("r#").unwrap_or(interface_name);
    let method = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}",
        interface.to_case(Case::Pascal),
        method.to_case(Case::Pascal)
    )
}

fn axum_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => rust_integer_type(value),
        hir::TypeSpec::FloatingPtType => "f64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "char".to_string(),
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "xidl_rust_axum::serde_json::Value".to_string()
        }
        hir::TypeSpec::ScopedName(value) => render_scoped_name(value),
        hir::TypeSpec::SequenceType(seq) => format!("Vec<{}>", axum_type(&seq.ty)),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "String".to_string(),
        hir::TypeSpec::FixedPtType(_) => "f64".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!(
                "::std::collections::BTreeMap<{}, {}>",
                axum_type(&map.key),
                axum_type(&map.value)
            )
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            rust_ident(&value.ident),
            value
                .args
                .iter()
                .map(axum_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn render_scoped_name(value: &hir::ScopedName) -> String {
    let mut iter = value.name.iter();
    let mut parts = Vec::new();
    if let Some(first) = iter.next() {
        if !value.is_root && first == "crate" {
            parts.push("crate".to_string());
        } else {
            parts.push(rust_ident(first));
        }
    }
    for part in iter {
        parts.push(rust_ident(part));
    }
    let path = parts.join("::");
    if value.is_root {
        format!("::{path}")
    } else {
        path
    }
}

fn rust_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "i8".to_string(),
        hir::IntegerType::UChar => "u8".to_string(),
        hir::IntegerType::U8 => "u8".to_string(),
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I8 => "i8".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}

#[allow(dead_code)]
fn default_path(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    format!("/{}", parts.join("/"))
}

#[allow(dead_code)]
fn attribute_path(attr_name: &str) -> String {
    format!("/attribute/{attr_name}")
}

#[allow(dead_code)]
fn route_from_annotations(
    annotations: &[hir::Annotation],
    default_method: HttpMethod,
) -> IdlcResult<(HttpMethod, Vec<String>)> {
    let mut verb_method = None;
    let mut paths = Vec::new();

    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if let Some(method) = method_from_annotation(annotation) {
            if let Some(prev) = verb_method {
                if prev != method {
                    return Err(IdlcError::rpc(
                        "more than one HTTP verb annotation is not allowed on a method",
                    ));
                }
            }
            verb_method = Some(method);
            if let Some(params) = annotation_params(annotation) {
                let params = normalize_params(params);
                if let Some(path) = params.get("path") {
                    paths.push(normalize_path(path));
                }
            }
            continue;
        }

        if name.eq_ignore_ascii_case("path") {
            if let Some(params) = annotation_params(annotation) {
                let params = normalize_params(params);
                if let Some(path) = params.get("value").or_else(|| params.get("path")) {
                    paths.push(normalize_path(path));
                }
            }
        }
    }

    let mut dedup = HashSet::new();
    paths.retain(|path| dedup.insert(path.clone()));
    Ok((verb_method.unwrap_or(default_method), paths))
}

#[allow(dead_code)]
fn method_from_annotation(annotation: &hir::Annotation) -> Option<HttpMethod> {
    let name = annotation_name(annotation)?;
    match name.to_ascii_lowercase().as_str() {
        "get" => Some(HttpMethod::Get),
        "post" => Some(HttpMethod::Post),
        "put" => Some(HttpMethod::Put),
        "patch" => Some(HttpMethod::Patch),
        "delete" => Some(HttpMethod::Delete),
        "head" => Some(HttpMethod::Head),
        "options" => Some(HttpMethod::Options),
        _ => None,
    }
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

#[allow(dead_code)]
fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

fn field_rename_raw(annotations: &[hir::Annotation]) -> Option<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("name") {
            continue;
        }
        let value = annotation_params(annotation)
            .map(normalize_params)
            .and_then(|params| {
                params
                    .get("value")
                    .cloned()
                    .or_else(|| params.get("name").cloned())
            });
        if value.is_some() {
            return value;
        }
    }
    None
}

fn field_rename(annotations: &[hir::Annotation], rust_name: &str) -> Option<String> {
    field_rename_raw(annotations).and_then(|value| serde_rename(&value, rust_name))
}

fn has_flatten_annotation(annotations: &[hir::Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("flatten"))
            .unwrap_or(false)
    })
}

#[allow(dead_code)]
struct SourceBinding {
    source: ParamSource,
    bound_name: String,
}

#[allow(dead_code)]
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
            .map(normalize_params)
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
            Some(ref prev) if prev.source == current && prev.bound_name == bound_name => {}
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

#[allow(dead_code)]
fn auto_default_method_path(op: &hir::OpDcl, method: HttpMethod) -> IdlcResult<String> {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let default_source = default_param_source(method);
    let mut path = normalize_path(&op.ident);
    for param in params {
        if matches!(param_direction(param.attr.as_ref()), ParamDirection::Out) {
            continue;
        }
        let binding = explicit_param_binding(param)?;
        let (source, bound_name) = match binding {
            Some(binding) => (binding.source, binding.bound_name),
            None => (default_source, param.declarator.0.clone()),
        };
        if matches!(source, ParamSource::Path) {
            path.push('/');
            path.push('{');
            path.push_str(&bound_name);
            path.push('}');
        }
    }
    Ok(path)
}

fn normalize_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match params {
        hir::AnnotationParams::Raw(value) => {
            for (key, value) in parse_raw_params(value) {
                out.insert(key.to_ascii_lowercase(), value);
            }
        }
        hir::AnnotationParams::Params(values) => {
            for value in values {
                let raw = value
                    .value
                    .as_ref()
                    .map(render_const_expr)
                    .unwrap_or_default();
                out.insert(
                    value.ident.to_ascii_lowercase(),
                    trim_quotes(&raw).unwrap_or(raw),
                );
            }
        }
        hir::AnnotationParams::ConstExpr(expr) => {
            let rendered = render_const_expr(expr);
            out.insert(
                "value".to_string(),
                trim_quotes(&rendered).unwrap_or(rendered),
            );
        }
    }
    out
}

fn parse_raw_params(raw: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            buf.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote.is_some() {
            escaped = true;
            buf.push(ch);
            continue;
        }
        match ch {
            '\'' | '"' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                buf.push(ch);
            }
            ',' if quote.is_none() => {
                let item = buf.trim();
                if !item.is_empty() {
                    parts.push(item.to_string());
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }

    let item = buf.trim();
    if !item.is_empty() {
        parts.push(item.to_string());
    }

    let mut out = Vec::new();
    for part in parts {
        if let Some((key, value)) = part.split_once('=') {
            let value = trim_quotes(value.trim()).unwrap_or_else(|| value.trim().to_string());
            out.push((key.trim().to_string(), unescape_param_value(&value)));
        }
    }
    out
}

fn unescape_param_value(value: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        out.push(ch);
    }
    out
}

fn trim_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(value[1..value.len() - 1].to_string());
        }
    }
    None
}

#[allow(dead_code)]
fn normalize_path(path: &str) -> String {
    let path = path.trim();
    let with_leading = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut collapsed = String::with_capacity(with_leading.len());
    let mut prev_slash = false;
    for ch in with_leading.chars() {
        if ch == '/' {
            if !prev_slash {
                collapsed.push(ch);
            }
            prev_slash = true;
        } else {
            collapsed.push(ch);
            prev_slash = false;
        }
    }
    if collapsed.len() > 1 && collapsed.ends_with('/') {
        collapsed.pop();
    }
    if collapsed.is_empty() {
        "/".to_string()
    } else {
        collapsed
    }
}

#[allow(dead_code)]
fn path_name_in_all_routes(name: &str, route_sets: &[HashSet<String>]) -> bool {
    route_sets.iter().all(|set| set.contains(name))
}

#[allow(dead_code)]
fn validate_head_constraints(op: &hir::OpDcl, method: HttpMethod) -> IdlcResult<()> {
    if !matches!(method, HttpMethod::Head) {
        return Ok(());
    }
    if !matches!(op.ty, hir::OpTypeSpec::Void) {
        return Err(IdlcError::rpc(format!(
            "HEAD method '{}' must return void",
            op.ident
        )));
    }
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    for param in params {
        if matches!(
            param_direction(param.attr.as_ref()),
            ParamDirection::Out | ParamDirection::InOut
        ) {
            return Err(IdlcError::rpc(format!(
                "HEAD method '{}' cannot contain out/inout parameter '{}'",
                op.ident, param.declarator.0
            )));
        }
    }
    Ok(())
}

fn render_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(
        expr,
        &crate::generate::rust::util::rust_scoped_name,
        &crate::generate::rust::util::rust_literal,
    )
}

fn serde_rename(raw: &str, rust: &str) -> Option<String> {
    let stripped = rust.strip_prefix("r#").unwrap_or(rust);
    if stripped == raw {
        None
    } else {
        Some(raw.to_string())
    }
}

#[allow(dead_code)]
fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut buf = String::new();
    let mut in_param = false;

    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
            }
            '}' if in_param => {
                if !buf.is_empty() {
                    out.insert(strip_path_param_prefix(&buf));
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

#[allow(dead_code)]
fn strip_path_param_prefix(value: &str) -> String {
    value.strip_prefix('*').unwrap_or(value).to_string()
}

fn path_param_is_catch_all(path: &str, name: &str) -> bool {
    path.contains(&format!("{{*{name}}}"))
}

#[allow(dead_code)]
fn validate_route_template(path: &str) -> IdlcResult<()> {
    let (path, _) = split_query_template(path)?;
    let mut start = 0usize;
    let mut catch_all_count = 0usize;
    while let Some(open_rel) = path[start..].find('{') {
        let open = start + open_rel;
        let close = path[open + 1..]
            .find('}')
            .map(|value| open + 1 + value)
            .ok_or_else(|| {
                IdlcError::rpc(format!("route template has unmatched '{{' in '{path}'"))
            })?;
        let token = &path[open + 1..close];
        let is_catch_all = token.starts_with('*');
        let name = token.strip_prefix('*').unwrap_or(token);
        if name.is_empty() {
            return Err(IdlcError::rpc(format!(
                "route template has empty path variable in '{path}'"
            )));
        }
        if is_catch_all {
            catch_all_count += 1;
            if catch_all_count > 1 {
                return Err(IdlcError::rpc(format!(
                    "route template contains more than one catch-all variable: '{path}'"
                )));
            }
            if close + 1 != path.len() {
                return Err(IdlcError::rpc(format!(
                    "catch-all variable must be at the end of route template: '{path}'"
                )));
            }
        }
        start = close + 1;
    }
    Ok(())
}

#[allow(dead_code)]
fn split_query_template(path: &str) -> IdlcResult<(String, HashSet<String>)> {
    let mut query_params = HashSet::new();
    if let Some(pos) = path.find("{?") {
        if !path.ends_with('}') {
            return Err(IdlcError::rpc(format!(
                "query template must terminate with '}}' in route '{path}'"
            )));
        }
        let tail = &path[pos + 2..path.len() - 1];
        if tail.trim().is_empty() {
            return Err(IdlcError::rpc(format!(
                "query template must include at least one variable in route '{path}'"
            )));
        }
        for name in tail.split(',').map(|value| value.trim()) {
            if name.is_empty() {
                return Err(IdlcError::rpc(format!(
                    "query template contains empty variable name in route '{path}'"
                )));
            }
            query_params.insert(name.to_string());
        }
        Ok((path[..pos].to_string(), query_params))
    } else {
        Ok((path.to_string(), query_params))
    }
}

#[allow(dead_code)]
fn parse_route_template(path: &str) -> IdlcResult<RouteTemplate> {
    validate_route_template(path)?;
    let (path, query_params) = split_query_template(path)?;
    let normalized = normalize_path(&path);
    let path_params = parse_path_params(&normalized);
    Ok(RouteTemplate {
        path: normalized,
        path_params,
        query_params,
    })
}

#[allow(dead_code)]
fn default_param_source(method: HttpMethod) -> ParamSource {
    match method {
        HttpMethod::Get | HttpMethod::Delete | HttpMethod::Head | HttpMethod::Options => {
            ParamSource::Query
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => ParamSource::Body,
    }
}

fn param_source_code(source: ParamSource) -> String {
    match source {
        ParamSource::Query => "ParamSource::Query".to_string(),
        ParamSource::Body => "ParamSource::Body".to_string(),
        ParamSource::Path => "ParamSource::Path".to_string(),
        ParamSource::Header => "ParamSource::Header".to_string(),
        ParamSource::Cookie => "ParamSource::Cookie".to_string(),
    }
}

fn header_is_multi(ty: &hir::TypeSpec) -> bool {
    matches!(ty, hir::TypeSpec::SequenceType(_))
}

fn header_item_ty(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SequenceType(seq) => axum_type(&seq.ty),
        _ => axum_type(ty),
    }
}

fn header_item_is_string(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::SequenceType(seq) => header_item_is_string(&seq.ty),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => true,
        _ => false,
    }
}

fn header_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::SequenceType(seq) => header_item_is_primitive(&seq.ty),
        hir::TypeSpec::IntegerType(_) | hir::TypeSpec::FloatingPtType | hir::TypeSpec::Boolean => {
            true
        }
        _ => false,
    }
}

fn cookie_is_multi(ty: &hir::TypeSpec) -> bool {
    header_is_multi(ty)
}

fn cookie_item_ty(ty: &hir::TypeSpec) -> String {
    header_item_ty(ty)
}

fn cookie_item_is_string(ty: &hir::TypeSpec) -> bool {
    header_item_is_string(ty)
}

fn cookie_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    header_item_is_primitive(ty)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

fn http_method_code(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "HttpMethod::Get".to_string(),
        HttpMethod::Post => "HttpMethod::Post".to_string(),
        HttpMethod::Put => "HttpMethod::Put".to_string(),
        HttpMethod::Patch => "HttpMethod::Patch".to_string(),
        HttpMethod::Delete => "HttpMethod::Delete".to_string(),
        HttpMethod::Head => "HttpMethod::Head".to_string(),
        HttpMethod::Options => "HttpMethod::Options".to_string(),
    }
}

fn http_method_fn(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "get".to_string(),
        HttpMethod::Post => "post".to_string(),
        HttpMethod::Put => "put".to_string(),
        HttpMethod::Patch => "patch".to_string(),
        HttpMethod::Delete => "delete".to_string(),
        HttpMethod::Head => "head".to_string(),
        HttpMethod::Options => "options".to_string(),
    }
}

fn reqwest_method_code(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "GET".to_string(),
        HttpMethod::Post => "POST".to_string(),
        HttpMethod::Put => "PUT".to_string(),
        HttpMethod::Patch => "PATCH".to_string(),
        HttpMethod::Delete => "DELETE".to_string(),
        HttpMethod::Head => "HEAD".to_string(),
        HttpMethod::Options => "OPTIONS".to_string(),
    }
}

#[allow(dead_code)]
fn method_name(method: HttpMethod) -> &'static str {
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
