use crate::error::{IdlcError, IdlcResult};
use crate::generate::http_hir::{
    HttpOperation,
    semantics::{
        HttpApiKeyLocation, HttpSecurityOrigin, HttpSecurityProfile, HttpSecurityRequirement,
        HttpStreamKind,
    },
};
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_axum::interface::context::{ApiKeyContext, MethodContext};
use crate::generate::rust_axum::interface::http::{
    deprecated_context_from_http, http_method_code, http_method_fn, http_method_from_hir,
    reqwest_method_code,
};
use crate::generate::rust_axum::interface::operation_params::process_operation_params;
use crate::generate::rust_axum::interface::types::{axum_type, method_struct_prefix};
use crate::generate::rust_axum::transport::{
    TransportDirection, TransportTracker, TypeRegistry, decode_expr, encode_expr,
};
use xidl_parser::hir;

struct SecurityContext {
    has_basic_auth: bool,
    has_bearer_auth: bool,
    auth_source_interface: bool,
    auth_source_method: bool,
    api_key_requirements: Vec<ApiKeyContext>,
    auth_ty: String,
}

pub fn render_op_from_http(
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
    let ret_in_expr = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => decode_expr("body", ty, registry)?,
    };
    let ret_out_expr = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => encode_expr("resp_value", ty, registry)?,
    };
    let return_is_unit = matches!(&op.ty, hir::OpTypeSpec::Void);

    let sec = process_security(http_op, &op.ident)?;
    let auth_ty = sec.auth_ty;

    let routes = &http_op.routes;
    let paths = routes.iter().map(|r| r.path.clone()).collect::<Vec<_>>();
    let path = paths
        .first()
        .cloned()
        .unwrap_or_else(|| format!("/{}", op.ident));

    let params_ctx = process_operation_params(op, http_op, &path, registry, transport)?;

    if (is_client_stream || is_bidi_stream)
        && (!params_ctx.path_params.is_empty()
            || !params_ctx.query_params.is_empty()
            || !params_ctx.header_params.is_empty()
            || !params_ctx.cookie_params.is_empty())
    {
        return Err(IdlcError::rpc(format!(
            "streaming method '{}' supports body parameters only",
            op.ident
        )));
    }
    let method = http_method_from_hir(http_op.method);
    let method_name = rust_ident(&op.ident);
    let auth_in_request_struct =
        (sec.has_basic_auth || sec.has_bearer_auth) && !(is_client_stream || is_bidi_stream);
    let auth_wrapper_struct =
        if (sec.has_basic_auth || sec.has_bearer_auth) && (is_client_stream || is_bidi_stream) {
            Some(format!(
                "{}AuthRequest",
                method_struct_prefix(interface_name, &op.ident)
            ))
        } else {
            None
        };
    let mut request_struct = if params_ctx.request_params.is_empty() {
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
    let mut param_list = params_ctx.param_list.clone();
    let param_names = params_ctx.param_names.clone();
    let mut server_params = params_ctx.param_list.clone();
    let mut server_param_names = params_ctx.param_names.clone();
    if sec.auth_source_method && (sec.has_basic_auth || sec.has_bearer_auth) {
        let name = "xidl_auth".to_string();
        param_list.push(format!("{name}: {auth_ty}"));
        auth_param = Some(name);
        auth_param_ty = auth_ty.clone();
    }
    if !is_client_stream && !is_bidi_stream && (sec.has_basic_auth || sec.has_bearer_auth) {
        let name = "xidl_auth".to_string();
        server_params.push(format!("{name}: {auth_ty}"));
        server_param_names.push(name);
    }
    let response_value_count = usize::from(!return_is_unit) + params_ctx.response_params.len();
    let response_body_count = usize::from(!return_is_unit) + params_ctx.response_body_params.len();
    let request_body_flatten =
        params_ctx.body_params.len() == 1 && params_ctx.body_params[0].flatten;
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
    let response_ty = if let Some(rs) = &response_struct {
        rs.clone()
    } else if !return_is_unit {
        ret.clone()
    } else if let Some(param) = params_ctx.response_params.first() {
        param.ty.clone()
    } else {
        "()".to_string()
    };
    let request_item_ty = request_ty.clone();
    let request_payload_ty = if is_client_stream {
        auth_wrapper_struct
            .clone()
            .unwrap_or_else(|| format!("xidl_rust_axum::stream::NdjsonStream<{}>", request_item_ty))
    } else if is_bidi_stream {
        auth_wrapper_struct.clone().unwrap_or_else(|| {
            format!(
                "xidl_rust_axum::stream::BidiServerStream<{}, {}>",
                request_item_ty, response_ty
            )
        })
    } else {
        request_ty.clone()
    };
    let basic_auth_realm = if sec.has_basic_auth {
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
        path_params: params_ctx.path_params,
        query_params: params_ctx.query_params,
        header_params: params_ctx.header_params,
        cookie_params: params_ctx.cookie_params,
        body_params: params_ctx.body_params,
        request_ty: request_ty.clone(),
        request_payload_ty,
        request_struct,
        auth_wrapper_struct,
        auth_in_request_struct,
        has_basic_auth: sec.has_basic_auth,
        has_bearer_auth: sec.has_bearer_auth,
        api_key_requirements: sec.api_key_requirements,
        auth_source_interface: sec.auth_source_interface,
        auth_source_method: sec.auth_source_method,
        auth_param,
        auth_param_ty,
        auth_ty,
        basic_auth_realm,
        request_params: params_ctx.request_params,
        response_struct,
        response_params: params_ctx.response_params,
        response_body_params: params_ctx.response_body_params,
        response_header_params: params_ctx.response_header_params,
        response_cookie_params: params_ctx.response_cookie_params,
        response_include_return,
        response_is_empty,
        return_is_unit,
        is_server_stream,
        is_client_stream,
        is_bidi_stream,
        request_item_ty,
        ret_in_ty,
        ret_out_ty,
        ret_in_expr,
        ret_out_expr,
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

fn process_security(http_op: &HttpOperation, op_ident: &str) -> IdlcResult<SecurityContext> {
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
                .any(|r| matches!(r, HttpSecurityRequirement::HttpBasic))
        })
        .unwrap_or(false);
    let has_bearer_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|r| matches!(r, HttpSecurityRequirement::HttpBearer))
        })
        .unwrap_or(false);
    let api_key_requirements = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .filter_map(|req| match req {
                    HttpSecurityRequirement::ApiKey { location, name } => Some(ApiKeyContext {
                        location: match location {
                            HttpApiKeyLocation::Header => "Header",
                            HttpApiKeyLocation::Query => "Query",
                            HttpApiKeyLocation::Cookie => "Cookie",
                        }
                        .to_string(),
                        name: name.clone(),
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if has_basic_auth && has_bearer_auth {
        return Err(IdlcError::rpc(format!(
            "operation '{op_ident}' cannot combine @http_basic and @http_bearer"
        )));
    }

    let auth_ty = if has_basic_auth {
        "xidl_rust_axum::auth::basic::BasicAuth".to_string()
    } else if has_bearer_auth {
        "xidl_rust_axum::auth::bearer::BearerAuth".to_string()
    } else {
        String::new()
    };

    Ok(SecurityContext {
        has_basic_auth,
        has_bearer_auth,
        auth_source_interface,
        auth_source_method,
        api_key_requirements,
        auth_ty,
    })
}
