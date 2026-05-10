use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_axum::interface::interface_http::{
    deprecated_context_from_http, http_method_code, http_method_fn, http_method_from_hir,
    reqwest_method_code, security_context,
};
use crate::generate::rust_axum::interface::interface_method_params::{
    MethodParams, collect_method_params,
};
use crate::generate::rust_axum::interface::interface_method_support::{
    ensure_streaming_constraints, method_struct_prefix, op_decode_expr, op_encode_expr,
    op_return_ty, request_payload_ty, response_ty,
};
use crate::generate::rust_axum::interface::{MethodContext, RenderEnv};
use crate::generate::rust_axum::transport::{TransportDirection, TransportTracker};
use xidl_parser::hir;
use xidl_parser::rest_hir::{
    HttpOperation, HttpRequestBodyShape, HttpResponseBodyShape, semantics::HttpStreamKind,
};

pub(crate) fn render_op_from_http(
    op: &hir::OpDcl,
    http_op: &HttpOperation,
    interface_name: &str,
    env: RenderEnv<'_>,
    transport: &mut TransportTracker,
) -> IdlcResult<MethodContext> {
    let stream = http_op.meta.stream;
    let is_server_stream = matches!(stream.kind, Some(HttpStreamKind::Server));
    let is_client_stream = matches!(stream.kind, Some(HttpStreamKind::Client));
    let is_bidi_stream = matches!(stream.kind, Some(HttpStreamKind::Bidi));
    let deprecated = deprecated_context_from_http(http_op);
    let security = security_context(&http_op.meta.security);
    if security.has_basic_auth && security.has_bearer_auth {
        return Err(IdlcError::rpc(format!(
            "operation '{}' cannot combine @http_basic and @http_bearer",
            op.ident
        )));
    }

    let ret = op_return_ty(&op.ty);
    let ret_in_ty = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => {
            transport.map_type(ty, TransportDirection::In, env.registry)?
        }
    };
    let ret_out_ty = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => {
            transport.map_type(ty, TransportDirection::Out, env.registry)?
        }
    };
    let ret_in_expr = op_decode_expr(&op.ty, env)?;
    let ret_out_expr = op_encode_expr(&op.ty, env)?;
    let return_is_unit = matches!(&op.ty, hir::OpTypeSpec::Void);

    let paths = http_op
        .meta
        .routes
        .iter()
        .map(|route| route.path.clone())
        .collect::<Vec<_>>();
    let path = paths
        .first()
        .cloned()
        .unwrap_or_else(|| format!("/{}", op.ident));
    let struct_prefix = method_struct_prefix(interface_name, &op.ident);
    let mut params = MethodParams::default();
    collect_method_params(op, http_op, &path, transport, env, &mut params)?;
    ensure_streaming_constraints(
        op,
        is_client_stream,
        is_bidi_stream,
        &params.path_params,
        &params.query_params,
        &params.header_params,
        &params.cookie_params,
    )?;

    let method = http_method_from_hir(http_op.meta.method);
    let method_name = rust_ident(&op.ident);
    let has_auth = security.has_basic_auth || security.has_bearer_auth;
    let auth_in_request_struct = has_auth && !(is_client_stream || is_bidi_stream);
    let auth_wrapper_struct = has_auth
        .then(|| {
            if is_client_stream || is_bidi_stream {
                Some(format!("{struct_prefix}AuthRequest"))
            } else {
                None
            }
        })
        .flatten();

    let has_inputs = !http_op.http.request.path.is_empty()
        || !http_op.http.request.query.is_empty()
        || !http_op.http.request.header.is_empty()
        || !http_op.http.request.cookie.is_empty()
        || !matches!(http_op.http.request.body.shape, HttpRequestBodyShape::Empty);

    let request_struct = if auth_in_request_struct || has_inputs {
        Some(format!("{struct_prefix}Request"))
    } else {
        None
    };

    let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
    let response_ty_str = response_ty(http_op, &struct_prefix, &ret);
    let request_payload_ty = request_payload_ty(
        &request_ty,
        &auth_wrapper_struct,
        is_client_stream,
        is_bidi_stream,
        &response_ty_str,
    );

    let mut auth_param = None;
    let mut auth_param_ty = String::new();
    let mut params_list = params.params.clone();
    let mut server_params = params.server_params.clone();
    let mut server_param_names = params.server_param_names.clone();
    if security.auth_source_method && has_auth {
        let name = "xidl_auth".to_string();
        params_list.push(format!("{name}: {}", security.auth_ty));
        auth_param = Some(name);
        auth_param_ty = security.auth_ty.clone();
    }
    if !is_client_stream && !is_bidi_stream && has_auth {
        let name = "xidl_auth".to_string();
        server_params.push(format!("{name}: {}", security.auth_ty));
        server_param_names.push(name);
    }

    let out_params_count = http_op
        .signature
        .params
        .iter()
        .filter(|p| {
            matches!(
                p.direction,
                xidl_parser::rest_hir::HttpSignatureParamDirection::Out
                    | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
            )
        })
        .count();
    let has_return = http_op.signature.return_type.is_some();
    let total_outputs = out_params_count + usize::from(has_return);

    let request_body_flatten = matches!(
        http_op.http.request.body.shape,
        HttpRequestBodyShape::SingleValue { flatten: true, .. }
    );
    let response_include_return = has_return;
    let response_is_empty = matches!(
        http_op.http.response.body.shape,
        HttpResponseBodyShape::Empty
    );
    let response_struct = if total_outputs > 1 || (!has_return && out_params_count == 1) {
        Some(format!("{struct_prefix}Response"))
    } else {
        None
    };
    let response_ty_str = response_ty(http_op, &struct_prefix, &ret);

    Ok(MethodContext {
        name: method_name.clone(),
        raw_name: op.ident.clone(),
        rust_attrs: rust_passthrough_attrs_from_annotations(&op.annotations),
        deprecated: deprecated.deprecated,
        deprecated_since: deprecated.since,
        deprecated_after: deprecated.after,
        deprecated_note: deprecated.note,
        params: params_list,
        param_names: params.param_names,
        server_params,
        server_param_names,
        ret,
        response_ty: response_ty_str,
        request_body_flatten,
        http_method: http_method_code(method),
        http_method_fn: http_method_fn(method),
        reqwest_method: reqwest_method_code(method),
        path,
        paths,
        struct_prefix,
        path_params: params.path_params,
        query_params: params.query_params,
        header_params: params.header_params,
        cookie_params: params.cookie_params,
        body_params: params.body_params,
        request_ty: request_ty.clone(),
        request_payload_ty,
        request_struct,
        auth_wrapper_struct,
        auth_in_request_struct,
        has_basic_auth: security.has_basic_auth,
        has_bearer_auth: security.has_bearer_auth,
        api_key_requirements: security.api_key_requirements,
        auth_source_interface: security.auth_source_interface,
        auth_source_method: security.auth_source_method,
        auth_param,
        auth_param_ty,
        auth_ty: security.auth_ty,
        basic_auth_realm: if security.has_basic_auth {
            http_op.meta.basic_auth_realm.clone().unwrap_or(method_name)
        } else {
            String::new()
        },
        request_params: params.request_params,
        response_struct,
        response_params: params.response_params,
        response_body_params: params.response_body_params,
        response_header_params: params.response_header_params,
        response_cookie_params: params.response_cookie_params,
        response_include_return,
        response_is_empty,
        return_is_unit,
        is_server_stream,
        is_client_stream,
        is_bidi_stream,
        request_item_ty: request_ty,
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
