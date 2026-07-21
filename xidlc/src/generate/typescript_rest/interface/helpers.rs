use super::super::model::{SecurityContext, ValueParamContext};
use crate::error::{IdlcError, IdlcResult};
use crate::generate::typescript::definition::TypeRefTarget;
use crate::generate::typescript::definition::contexts::{
    ClientParamContext, ParamDeclContext, TsType,
};
use crate::generate::typescript::definition::names::{scoped_name, ts_ident, ts_prop_name};
use crate::generate::typescript::definition::type_expr::{
    ts_type_for_type_spec, zod_schema_for_type_spec_with_prefix,
};
use xidl_parser::hir;
use xidl_parser::rest_hir::{
    HttpInputBinding, HttpOperation, HttpOutputBinding, HttpOutputSource, HttpRequestBodyShape,
    HttpResponseBodyShape,
    semantics::{HttpSecurityRequirement, HttpStreamCodec, HttpStreamKind},
};

pub(super) fn build_response_fields(
    op: &HttpOperation,
    module_path: &[String],
) -> Vec<ParamDeclContext> {
    let mut fields = Vec::new();
    if let Some(ty) = &op.signature.return_type {
        fields.push(ParamDeclContext {
            prop: "return".to_string(),
            ty: ts_type_for_type_spec(ty, module_path, TypeRefTarget::Types),
            schema: zod_schema_for_type_spec_with_prefix(ty, module_path, Some("models")),
            optional: false,
            doc: Vec::new(),
        });
    }
    for param in &op.signature.params {
        if matches!(
            param.direction,
            xidl_parser::rest_hir::HttpSignatureParamDirection::Out
                | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
        ) {
            fields.push(ParamDeclContext {
                prop: ts_prop_name(&param.name),
                ty: ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Types),
                schema: zod_schema_for_type_spec_with_prefix(
                    &param.ty,
                    module_path,
                    Some("models"),
                ),
                optional: param.annotations.iter().any(|a| {
                    matches!(
                        a,
                        xidl_parser::rest_hir::HttpSignatureParamAnnotation::Optional
                    )
                }),
                doc: Vec::new(),
            });
        }
    }
    fields
}

pub(super) fn build_client_params(
    op: &HttpOperation,
    module_path: &[String],
    request_name: &Option<String>,
) -> Vec<ClientParamContext> {
    if matches!(op.meta.stream.kind, Some(HttpStreamKind::Client)) {
        return vec![ClientParamContext {
            name: "stream".to_string(),
            ty: build_client_stream_ty(op, module_path, request_name).unwrap_or(TsType::Void),
        }];
    }
    op.signature
        .params
        .iter()
        .filter(|p| {
            matches!(
                p.direction,
                xidl_parser::rest_hir::HttpSignatureParamDirection::In
                    | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
            )
        })
        .map(|p| {
            let base = ts_type_for_type_spec(&p.ty, module_path, TypeRefTarget::Client);
            let optional = p.annotations.iter().any(|a| {
                matches!(
                    a,
                    xidl_parser::rest_hir::HttpSignatureParamAnnotation::Optional
                )
            });
            let ty = if optional {
                TsType::Optional(Box::new(base))
            } else {
                base
            };
            ClientParamContext {
                name: ts_ident(&p.name),
                ty,
            }
        })
        .collect()
}

pub(super) fn build_value_params(bindings: &[HttpInputBinding]) -> Vec<ValueParamContext> {
    bindings
        .iter()
        .map(|b| ValueParamContext {
            raw_name: b.wire_name.clone(),
            access: ts_ident(&b.source_param),
            key_name: b.source_param.clone(),
            optional: b.optional,
            is_multi: matches!(b.ty, hir::TypeSpec::SequenceType(_)),
        })
        .collect()
}

pub(super) fn build_response_value_params(
    bindings: &[HttpOutputBinding],
) -> Vec<ValueParamContext> {
    bindings
        .iter()
        .map(|b| {
            let name = match &b.source {
                HttpOutputSource::ReturnValue => "return".to_string(),
                HttpOutputSource::Param { name } => name.clone(),
            };
            ValueParamContext {
                raw_name: b.wire_name.clone(),
                access: ts_prop_name(&name),
                key_name: name,
                optional: false,
                is_multi: matches!(b.ty, hir::TypeSpec::SequenceType(_)),
            }
        })
        .collect()
}

pub(super) fn build_server_stream_ty(op: &HttpOperation, module_path: &[String]) -> Option<TsType> {
    if let HttpResponseBodyShape::Stream { item_ty, .. } = &op.http.response.body.shape {
        Some(ts_type_for_type_spec(
            item_ty,
            module_path,
            TypeRefTarget::Client,
        ))
    } else {
        None
    }
}

pub(super) fn build_client_stream_ty(
    op: &HttpOperation,
    module_path: &[String],
    request_name: &Option<String>,
) -> Option<TsType> {
    if let HttpRequestBodyShape::Stream {
        item_ty,
        source_param,
        ..
    } = &op.http.request.body.shape
    {
        let was_flattened = op.signature.params.iter().any(|p| {
            p.name == *source_param
                && p.annotations.iter().any(|a| {
                    matches!(
                        a,
                        xidl_parser::rest_hir::HttpSignatureParamAnnotation::Flatten
                    )
                })
        });

        let is_byte_stream =
            op.http.request.body.content_type.as_deref() == Some("application/octet-stream");
        let item_ty_str = if was_flattened || is_byte_stream {
            ts_type_for_type_spec(item_ty, module_path, TypeRefTarget::Client)
        } else if let Some(name) = request_name {
            TsType::ScopedName(format!("ifaceTypes.{}", scoped_name(module_path, name)))
        } else {
            TsType::Void
        };
        Some(TsType::AsyncIterable(Box::new(item_ty_str)))
    } else {
        None
    }
}

pub(super) fn build_response_body_mode(op: &HttpOperation) -> &'static str {
    match &op.http.response.body.shape {
        HttpResponseBodyShape::Empty => "none",
        HttpResponseBodyShape::ReturnOnly { .. } => "return",
        HttpResponseBodyShape::SingleValue { source, .. } => {
            if matches!(source, HttpOutputSource::ReturnValue) {
                "return"
            } else {
                "object"
            }
        }
        HttpResponseBodyShape::Object { .. } => "object",
        HttpResponseBodyShape::Stream { .. } => "return",
    }
}

pub(super) fn validate_stream_support(op: &HttpOperation) -> IdlcResult<()> {
    match op.meta.stream.kind {
        Some(HttpStreamKind::Server) if op.meta.stream.codec != HttpStreamCodec::Sse => {
            Err(IdlcError::rpc(format!(
                "typescript-rest currently supports only SSE for @server_stream methods: '{}'",
                op.meta.name
            )))
        }
        Some(HttpStreamKind::Client) if op.meta.stream.codec != HttpStreamCodec::Ndjson => {
            Err(IdlcError::rpc(format!(
                "typescript-rest currently supports only NDJSON for @client_stream methods: '{}'",
                op.meta.name
            )))
        }
        Some(HttpStreamKind::Bidi) => Err(IdlcError::rpc(format!(
            "typescript-rest currently does not support @bidi_stream methods: '{}'",
            op.meta.name
        ))),
        _ => Ok(()),
    }
}

pub(super) fn only_direct_return(op: &HttpOperation) -> bool {
    op.signature.return_type.is_some()
        && op.http.response.header.is_empty()
        && op.http.response.cookie.is_empty()
        && matches!(
            op.http.response.body.shape,
            HttpResponseBodyShape::ReturnOnly { .. }
                | HttpResponseBodyShape::Stream {
                    item_source: HttpOutputSource::ReturnValue,
                    ..
                }
        )
}

pub(super) fn http_method_name(method: xidl_parser::rest_hir::HttpMethod) -> &'static str {
    match method {
        xidl_parser::rest_hir::HttpMethod::Get => "GET",
        xidl_parser::rest_hir::HttpMethod::Post => "POST",
        xidl_parser::rest_hir::HttpMethod::Put => "PUT",
        xidl_parser::rest_hir::HttpMethod::Patch => "PATCH",
        xidl_parser::rest_hir::HttpMethod::Delete => "DELETE",
        xidl_parser::rest_hir::HttpMethod::Head => "HEAD",
        xidl_parser::rest_hir::HttpMethod::Options => "OPTIONS",
    }
}

pub(super) fn security_contexts(op: &HttpOperation) -> Vec<SecurityContext> {
    op.meta
        .security
        .as_ref()
        .map(|profile| {
            profile
                .requirements
                .iter()
                .map(|value| match value {
                    HttpSecurityRequirement::HttpBasic => SecurityContext {
                        kind: "http_basic".to_string(),
                        location: None,
                        name: None,
                        realm: op.meta.basic_auth_realm.clone(),
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::HttpBearer => SecurityContext {
                        kind: "http_bearer".to_string(),
                        location: None,
                        name: None,
                        realm: None,
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::ApiKey { location, name } => SecurityContext {
                        kind: "api_key".to_string(),
                        location: Some(format!("{location:?}").to_ascii_lowercase()),
                        name: Some(name.clone()),
                        realm: None,
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::OAuth2 { scopes } => SecurityContext {
                        kind: "oauth2".to_string(),
                        location: None,
                        name: None,
                        realm: None,
                        scopes: scopes.clone(),
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}
