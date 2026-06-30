use crate::error::IdlcResult;
use crate::generate::go_rest::{MethodMeta, ParamMeta, ParamSource, definition};
use convert_case::{Case, Casing};
use xidl_parser::rest_hir::{
    HttpOperation, HttpOutputSource, HttpRequestBodyShape, HttpResponseBodyShape,
};

use super::interface_meta_support::{deprecated_context, http_method, validate_stream_support};

pub(crate) fn build_method_meta(
    interface_name: &str,
    op: &HttpOperation,
) -> IdlcResult<MethodMeta> {
    validate_stream_support(op)?;
    let http_method = http_method(op.meta.method);
    let deprecated = deprecated_context(op);
    let (security, basic_realm) = match &op.meta.security {
        None => (Vec::new(), None),
        Some(profile) => (
            profile.requirements.clone(),
            op.meta.basic_auth_realm.clone(),
        ),
    };

    let mut request_params = Vec::new();
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut header_params = Vec::new();
    let mut cookie_params = Vec::new();
    let mut body_params = Vec::new();

    for p in &op.signature.params {
        if matches!(
            p.direction,
            xidl_parser::rest_hir::HttpSignatureParamDirection::In
                | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
        ) {
            let meta = build_param_meta(op, &p.name);
            request_params.push(meta.clone());
            match meta.source {
                ParamSource::Path => path_params.push(meta),
                ParamSource::Query => query_params.push(meta),
                ParamSource::Header => header_params.push(meta),
                ParamSource::Cookie => cookie_params.push(meta),
                ParamSource::Body => body_params.push(meta),
            }
        }
    }

    let mut response_body_params = Vec::new();
    let mut response_header_params = Vec::new();
    let mut response_cookie_params = Vec::new();

    for p in &op.signature.params {
        if matches!(
            p.direction,
            xidl_parser::rest_hir::HttpSignatureParamDirection::Out
                | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
        ) {
            let meta = build_response_param_meta(op, &p.name);
            match meta.source {
                ParamSource::Header => response_header_params.push(meta),
                ParamSource::Cookie => response_cookie_params.push(meta),
                _ => response_body_params.push(meta),
            }
        }
    }

    let struct_prefix = format!("{}{}", interface_name, op.meta.name.to_case(Case::Pascal));
    let request_struct = format!("{struct_prefix}Request");
    let response_struct = format!("{struct_prefix}Response");

    let (request_body_struct, request_body_direct_field, request_body_direct_ty) =
        match &op.http.request.body.shape {
            HttpRequestBodyShape::Object { .. } => {
                (Some(format!("{struct_prefix}RequestBody")), None, None)
            }
            HttpRequestBodyShape::SingleValue {
                source_param,
                flatten,
                ty,
            } => {
                let is_text = matches!(
                    op.http.request.body.codec,
                    Some(xidl_parser::rest_hir::HttpBodyCodec::Text)
                );
                if *flatten || is_text {
                    (
                        None,
                        Some(source_param.to_case(Case::Pascal)),
                        Some(definition::go_type(ty)),
                    )
                } else {
                    (Some(format!("{struct_prefix}RequestBody")), None, None)
                }
            }
            _ => (None, None, None),
        };

    let return_ty = op.signature.return_type.as_ref().map(definition::go_type);

    let (response_body_struct, response_body_direct_field, response_body_direct_ty) =
        match &op.http.response.body.shape {
            HttpResponseBodyShape::Object { .. } => {
                (Some(format!("{struct_prefix}ResponseBody")), None, None)
            }
            HttpResponseBodyShape::ReturnOnly { ty } => (
                None,
                Some("Return".to_string()),
                Some(definition::go_type(ty)),
            ),
            HttpResponseBodyShape::SingleValue { source, ty } => {
                let name = match source {
                    HttpOutputSource::ReturnValue => "Return".to_string(),
                    HttpOutputSource::Param { name } => name.to_case(Case::Pascal),
                };
                (None, Some(name), Some(definition::go_type(ty)))
            }
            _ => (None, None, None),
        };

    Ok(MethodMeta {
        method_name: op.meta.name.to_case(Case::Pascal),
        struct_prefix,
        http_method,
        paths: op
            .meta
            .routes
            .iter()
            .map(|item| item.path.clone())
            .collect(),
        request_struct,
        request_body_struct,
        request_body_direct_field,
        request_body_direct_ty,
        response_struct,
        response_body_struct,
        response_body_direct_field,
        response_body_direct_ty,
        request_content_type: op
            .http
            .request
            .body
            .content_type
            .clone()
            .unwrap_or_default(),
        request_content_type_explicit: op.http.request.body.content_type_explicit,
        response_content_type: op
            .http
            .response
            .body
            .content_type
            .clone()
            .unwrap_or_default(),
        response_content_type_explicit: op.http.response.body.content_type_explicit,
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
        stream_kind: op.meta.stream.kind,
        stream_codec: op.meta.stream.codec,
        cors: op.meta.cors.clone(),
        security,
        basic_realm,
        deprecated: deprecated.deprecated,
        deprecated_since: deprecated.since,
        deprecated_after: deprecated.after,
        deprecated_note: deprecated.note,
    })
}

fn build_param_meta(op: &HttpOperation, name: &str) -> ParamMeta {
    let p = op.signature.params.iter().find(|p| p.name == name).unwrap();
    let optional = p.is_optional;
    let flatten = p.is_flatten;

    let (source, wire_name) =
        if let Some(b) = op.http.request.path.iter().find(|b| b.source_param == name) {
            (ParamSource::Path, b.wire_name.clone())
        } else if let Some(b) = op
            .http
            .request
            .query
            .iter()
            .find(|b| b.source_param == name)
        {
            (ParamSource::Query, b.wire_name.clone())
        } else if let Some(b) = op
            .http
            .request
            .header
            .iter()
            .find(|b| b.source_param == name)
        {
            (ParamSource::Header, b.wire_name.clone())
        } else if let Some(b) = op
            .http
            .request
            .cookie
            .iter()
            .find(|b| b.source_param == name)
        {
            (ParamSource::Cookie, b.wire_name.clone())
        } else {
            match &op.http.request.body.shape {
                HttpRequestBodyShape::Object { fields } => {
                    let f = fields.iter().find(|f| f.source_param == name).unwrap();
                    (ParamSource::Body, f.field_name.clone())
                }
                HttpRequestBodyShape::SingleValue {
                    source_param: _, ..
                } => {
                    let is_text = matches!(
                        op.http.request.body.codec,
                        Some(xidl_parser::rest_hir::HttpBodyCodec::Text)
                    );
                    (
                        ParamSource::Body,
                        if flatten || is_text {
                            "".to_string()
                        } else {
                            name.to_string()
                        },
                    )
                }
                _ => (ParamSource::Body, "".to_string()),
            }
        };

    ParamMeta {
        field_name: name.to_case(Case::Pascal),
        raw_name: name.to_string(),
        wire_name,
        ty: definition::go_type(&p.ty),
        optional,
        source,
    }
}

fn build_response_param_meta(op: &HttpOperation, name: &str) -> ParamMeta {
    let p = op.signature.params.iter().find(|p| p.name == name).unwrap();
    let (source, wire_name) = if let Some(b) =
        op.http.response.header.iter().find(|b| match &b.source {
            HttpOutputSource::Param { name: n } => n == name,
            _ => false,
        }) {
        (ParamSource::Header, b.wire_name.clone())
    } else if let Some(b) = op.http.response.cookie.iter().find(|b| match &b.source {
        HttpOutputSource::Param { name: n } => n == name,
        _ => false,
    }) {
        (ParamSource::Cookie, b.wire_name.clone())
    } else {
        (ParamSource::Body, name.to_string())
    };

    ParamMeta {
        field_name: name.to_case(Case::Pascal),
        raw_name: name.to_string(),
        wire_name,
        ty: definition::go_type(&p.ty),
        optional: false,
        source,
    }
}
