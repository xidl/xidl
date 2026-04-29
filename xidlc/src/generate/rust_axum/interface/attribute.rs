use crate::error::IdlcResult;
use crate::generate::http_hir::{
    HttpOperation, HttpParamKind,
    semantics::{
        HttpApiKeyLocation, HttpSecurityOrigin, HttpSecurityProfile, HttpSecurityRequirement,
        HttpStreamKind,
    },
};
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_axum::interface::context::{ApiKeyContext, MethodContext, ParamContext};
use crate::generate::rust_axum::interface::http::{
    deprecated_context_from_http, http_method_code, http_method_fn, http_method_from_hir,
    reqwest_method_code,
};
use crate::generate::rust_axum::interface::params::{ParamSource, param_source_code};
use crate::generate::rust_axum::interface::types::{axum_type, method_struct_prefix};
use crate::generate::rust_axum::transport::{
    TransportDirection, TransportTracker, TypeRegistry, decode_expr, encode_expr,
};
use xidl_parser::hir;

pub fn render_attr_operation_from_http(
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
        .find(|param| matches!(param.kind, HttpParamKind::Body))
    {
        let param_name = rust_ident(&param.name);
        let ty = axum_type(&param.ty);
        params.push(format!("{param_name}: {ty}"));
        param_names.push(param_name.clone());
        server_params.push(format!("{param_name}: {ty}"));
        server_param_names.push(param_name.clone());
        let request_param = ParamContext {
            name: param_name.clone(),
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
            in_expr: decode_expr(&param_name, &param.ty, registry)?,
            out_expr: encode_expr(&param_name, &param.ty, registry)?,
            field_in_expr: decode_expr(&format!("value.{param_name}"), &param.ty, registry)?,
            field_out_expr: encode_expr(&format!("value.{param_name}"), &param.ty, registry)?,
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
    let ret_in_expr = match &http_op.return_type {
        Some(ty) => decode_expr("body", ty, registry)?,
        None => "()".to_string(),
    };
    let ret_out_expr = match &http_op.return_type {
        Some(ty) => encode_expr("resp_value", ty, registry)?,
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
        ret_in_expr,
        ret_out_expr,
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

pub fn attr_operation_names(attr: &hir::AttrDcl) -> Vec<String> {
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
