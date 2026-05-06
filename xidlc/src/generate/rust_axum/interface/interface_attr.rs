use crate::error::IdlcResult;
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_axum::interface::interface_http::{
    deprecated_context_from_http, http_method_code, http_method_fn, http_method_from_hir,
    reqwest_method_code, security_context,
};
use crate::generate::rust_axum::interface::interface_types::axum_type;
use crate::generate::rust_axum::interface::{MethodContext, ParamContext, ParamSource, RenderEnv};
use crate::generate::rust_axum::transport::{
    TransportDirection, TransportTracker, decode_expr, encode_expr,
};
use xidl_parser::hir;
use xidl_parser::rest_hir::{
    HttpOperation, HttpParamKind as RestHirParamKind, semantics::HttpStreamKind,
};

pub(crate) fn render_attr_operation_from_http(
    attr: Option<&hir::AttrDcl>,
    http_op: &HttpOperation,
    interface_name: &str,
    env: RenderEnv<'_>,
    transport: &mut TransportTracker,
) -> IdlcResult<MethodContext> {
    let rust_attrs = attr
        .map(|attr| rust_passthrough_attrs_from_annotations(&attr.annotations))
        .unwrap_or_default();
    let deprecated = deprecated_context_from_http(http_op);
    let security = security_context(&http_op.security);
    let has_auth = security.has_basic_auth || security.has_bearer_auth;
    let request_struct = if has_auth || !http_op.request_params.is_empty() {
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
        .find(|param| matches!(param.kind, RestHirParamKind::Body))
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
            in_ty: transport.map_type(&param.ty, TransportDirection::In, env.registry)?,
            out_ty: transport.map_type(&param.ty, TransportDirection::Out, env.registry)?,
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
            in_expr: decode_expr(&param_name, &param.ty, env.registry)?,
            out_expr: encode_expr(&param_name, &param.ty, env.registry)?,
            field_in_expr: decode_expr(&format!("value.{param_name}"), &param.ty, env.registry)?,
            field_out_expr: encode_expr(&format!("value.{param_name}"), &param.ty, env.registry)?,
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
        Some(ty) => transport.map_type(ty, TransportDirection::In, env.registry)?,
        None => "()".to_string(),
    };
    let ret_out_ty = match &http_op.return_type {
        Some(ty) => transport.map_type(ty, TransportDirection::Out, env.registry)?,
        None => "()".to_string(),
    };
    let ret_in_expr = match &http_op.return_type {
        Some(ty) => decode_expr("body", ty, env.registry)?,
        None => "()".to_string(),
    };
    let ret_out_expr = match &http_op.return_type {
        Some(ty) => encode_expr("resp_value", ty, env.registry)?,
        None => "()".to_string(),
    };
    let return_is_unit = http_op.return_type.is_none();
    if has_auth {
        let name = "xidl_auth".to_string();
        server_params.push(format!("{name}: {}", security.auth_ty));
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
        has_basic_auth: security.has_basic_auth,
        has_bearer_auth: security.has_bearer_auth,
        api_key_requirements: security.api_key_requirements,
        auth_source_interface: security.auth_source_interface,
        auth_source_method: security.auth_source_method,
        auth_param: None,
        auth_param_ty: String::new(),
        auth_ty: security.auth_ty,
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

fn method_struct_prefix(interface_name: &str, method_name: &str) -> String {
    use convert_case::{Case, Casing};

    let interface = interface_name.strip_prefix("r#").unwrap_or(interface_name);
    let method = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}",
        interface.to_case(Case::Pascal),
        method.to_case(Case::Pascal)
    )
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
