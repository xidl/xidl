use crate::error::{IdlcError, IdlcResult};
use crate::generate::http_hir::{
    HttpMethod as HttpHirMethod, HttpOperation, HttpOperationSource, HttpParam as HttpHirParam,
    HttpParamKind as HttpHirParamKind,
    semantics::{
        HttpApiKeyLocation, HttpSecurityOrigin, HttpSecurityProfile, HttpSecurityRequirement,
        HttpStreamKind, has_optional_annotation,
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

fn path_param_is_catch_all(path: &str, name: &str) -> bool {
    path.contains(&format!("{{*{name}}}"))
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
