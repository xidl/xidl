use crate::error::IdlcResult;
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_axum::interface::interface_http::{
    cors_layer, deprecated_context_from_http, http_method_code, http_method_fn,
    http_method_from_hir, reqwest_method_code, security_context,
};
use crate::generate::rust_axum::interface::interface_method_params::find_input_binding;
use crate::generate::rust_axum::interface::interface_types::axum_type;
use crate::generate::rust_axum::interface::{MethodContext, ParamContext, ParamSource, RenderEnv};
use crate::generate::rust_axum::transport::{
    TransportDirection, TransportTracker, decode_expr, encode_expr,
};
use xidl_parser::hir;
use xidl_parser::rest_hir::{HttpOperation, semantics::HttpStreamKind};

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
    let security = security_context(&http_op.meta.security);
    let has_auth = security.has_basic_auth || security.has_bearer_auth;

    let has_inputs = !http_op.signature.params.is_empty()
        || matches!(
            http_op.meta.stream.kind,
            Some(HttpStreamKind::Client | HttpStreamKind::Bidi)
        );

    let request_struct = if has_auth || has_inputs {
        Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &http_op.meta.name)
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

    for p in &http_op.signature.params {
        if matches!(
            p.direction,
            xidl_parser::rest_hir::HttpSignatureParamDirection::In
                | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
        ) {
            let param_name = rust_ident(&p.name);
            let ty = axum_type(&p.ty);
            params.push(format!("{param_name}: {ty}"));
            param_names.push(param_name.clone());
            server_params.push(format!("{param_name}: {ty}"));
            server_param_names.push(param_name.clone());

            let (_, wire_name) = find_input_binding(http_op, &p.name, false);

            let request_param = ParamContext {
                name: param_name.clone(),
                raw_name: p.name.clone(),
                wire_name,
                path_template_name: String::new(),
                ty: ty.clone(),
                in_ty: transport.map_type(&p.ty, TransportDirection::In, env.registry)?,
                out_ty: transport.map_type(&p.ty, TransportDirection::Out, env.registry)?,
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
                in_expr: decode_expr(&param_name, &p.ty, env.registry)?,
                out_expr: encode_expr(&param_name, &p.ty, env.registry)?,
                field_in_expr: decode_expr(&format!("value.{param_name}"), &p.ty, env.registry)?,
                field_out_expr: encode_expr(&format!("value.{param_name}"), &p.ty, env.registry)?,
            };
            request_params.push(request_param.clone());
            body_params.push(request_param);
        }
    }

    let ret = http_op
        .signature
        .return_type
        .as_ref()
        .map(axum_type)
        .unwrap_or_else(|| "()".to_string());
    let ret_in_ty = match &http_op.signature.return_type {
        Some(ty) => transport.map_type(ty, TransportDirection::In, env.registry)?,
        None => "()".to_string(),
    };
    let ret_out_ty = match &http_op.signature.return_type {
        Some(ty) => transport.map_type(ty, TransportDirection::Out, env.registry)?,
        None => "()".to_string(),
    };
    let ret_in_expr = match &http_op.signature.return_type {
        Some(ty) => decode_expr("body", ty, env.registry)?,
        None => "()".to_string(),
    };
    let ret_out_expr = match &http_op.signature.return_type {
        Some(ty) => encode_expr("resp_value", ty, env.registry)?,
        None => "()".to_string(),
    };
    let return_is_unit = http_op.signature.return_type.is_none();
    if has_auth {
        let name = "xidl_auth".to_string();
        server_params.push(format!("{name}: {}", security.auth_ty));
        server_param_names.push(name);
    }

    Ok(MethodContext {
        name: rust_ident(&http_op.meta.name),
        raw_name: http_op.meta.name.clone(),
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
        request_body_flatten: matches!(
            http_op.http.request.body.shape,
            xidl_parser::rest_hir::HttpRequestBodyShape::SingleValue { flatten, .. }
                if flatten
                    || matches!(
                        http_op.http.request.body.codec,
                        Some(xidl_parser::rest_hir::HttpBodyCodec::Text)
                    )
        ),
        http_method: http_method_code(http_method_from_hir(http_op.meta.method)),
        http_method_fn: http_method_fn(http_method_from_hir(http_op.meta.method)),
        reqwest_method: reqwest_method_code(http_method_from_hir(http_op.meta.method)),
        paths: http_op
            .meta
            .routes
            .iter()
            .map(|route| route.path.clone())
            .collect(),
        path: http_op
            .meta
            .routes
            .first()
            .map(|route| route.path.clone())
            .unwrap_or_default(),
        cors_layer: cors_layer(http_op),
        struct_prefix: method_struct_prefix(interface_name, &http_op.meta.name),
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
        basic_auth_realm: http_op.meta.basic_auth_realm.clone().unwrap_or_default(),
        request_params,
        response_struct: None,
        response_params: Vec::new(),
        response_body_params: Vec::new(),
        response_header_params: Vec::new(),
        response_cookie_params: Vec::new(),
        response_include_return: !return_is_unit,
        response_is_empty: return_is_unit,
        return_is_unit,
        is_server_stream: matches!(http_op.meta.stream.kind, Some(HttpStreamKind::Server)),
        is_client_stream: matches!(http_op.meta.stream.kind, Some(HttpStreamKind::Client)),
        is_bidi_stream: matches!(http_op.meta.stream.kind, Some(HttpStreamKind::Bidi)),
        is_upgrade: false,
        upgrade_protocol: None,
        request_item_ty: "()".to_string(),
        ret_in_ty,
        ret_out_ty,
        ret_in_expr,
        ret_out_expr,
        request_content_type: http_op
            .http
            .request
            .body
            .content_type
            .clone()
            .unwrap_or_else(|| "application/json".to_string()),
        response_content_type: http_op
            .http
            .response
            .body
            .content_type
            .clone()
            .unwrap_or_else(|| "application/json".to_string()),
        response_status: http_op.http.response.status.clone(),
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
