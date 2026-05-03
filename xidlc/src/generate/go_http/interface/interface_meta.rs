use crate::error::IdlcResult;
use crate::generate::go_http::{MethodMeta, ParamSource, definition};
use xidl_parser::http_hir::{
    HttpOperation,
    semantics::{HttpStreamCodec, HttpStreamKind},
};
use convert_case::{Case, Casing};

use super::interface_meta_support::{
    deprecated_context, http_method, param_meta, request_body_context, response_body_context,
    response_params_from_meta, validate_stream_support,
};

pub(crate) fn build_method_meta(
    interface_name: &str,
    op: &HttpOperation,
) -> IdlcResult<MethodMeta> {
    validate_stream_support(op)?;
    let http_method = http_method(op.method);
    let deprecated = deprecated_context(op);
    let (security, basic_realm) = match &op.security {
        None => (Vec::new(), None),
        Some(profile) => (profile.requirements.clone(), op.basic_auth_realm.clone()),
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

    for param in &op.request_params {
        let meta = param_meta(param);
        request_params.push(meta.clone());
        match meta.source {
            ParamSource::Path => path_params.push(meta),
            ParamSource::Query => query_params.push(meta),
            ParamSource::Header => header_params.push(meta),
            ParamSource::Cookie => cookie_params.push(meta),
            ParamSource::Body => body_params.push(meta),
        }
    }
    for param in &op.response_params {
        let meta = param_meta(param);
        response_params_from_meta(
            &mut response_body_params,
            &mut response_header_params,
            &mut response_cookie_params,
            meta,
        );
    }

    let struct_prefix = format!("{}{}", interface_name, op.name.to_case(Case::Pascal));
    let request_struct = format!("{struct_prefix}Request");
    let response_struct = format!("{struct_prefix}Response");
    let (request_body_struct, request_body_direct_field, request_body_direct_ty) =
        request_body_context(&body_params, &struct_prefix, op.stream.kind);
    let return_ty = op.return_type.as_ref().map(definition::go_type);
    let (response_body_struct, response_body_direct_field, response_body_direct_ty) =
        response_body_context(&response_body_params, &return_ty, &struct_prefix);

    Ok(MethodMeta {
        method_name: op.name.to_case(Case::Pascal),
        struct_prefix,
        http_method,
        paths: op.routes.iter().map(|item| item.path.clone()).collect(),
        request_struct,
        request_body_struct,
        request_body_direct_field,
        request_body_direct_ty,
        response_struct,
        response_body_struct,
        response_body_direct_field,
        response_body_direct_ty,
        request_content_type: if matches!(op.stream.kind, Some(HttpStreamKind::Client)) {
            "application/x-ndjson".to_string()
        } else {
            op.request_content_type.clone()
        },
        response_content_type: if matches!(op.stream.kind, Some(HttpStreamKind::Server))
            && op.stream.codec == HttpStreamCodec::Sse
        {
            "text/event-stream".to_string()
        } else if matches!(op.stream.kind, Some(HttpStreamKind::Client)) {
            "application/json".to_string()
        } else {
            op.response_content_type.clone()
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
        stream_kind: op.stream.kind,
        stream_codec: op.stream.codec,
        security,
        basic_realm,
        deprecated: deprecated.deprecated,
        deprecated_since: deprecated.since,
        deprecated_after: deprecated.after,
        deprecated_note: deprecated.note,
    })
}
