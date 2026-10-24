use super::semantics::{HttpStreamConfig, HttpStreamKind};
use super::validate::{HttpParamDirection, param_direction};
use super::*;
use crate::hir;

#[cfg(test)]
mod tests;
mod utils;
use utils::*;

/// Builds signature metadata using the same explicit bindings as HTTP projection.
pub fn build_operation_signature(op: &hir::OpDcl) -> Result<HttpOperationSignature, String> {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);

    let sig_params = params
        .iter()
        .map(|param| {
            let direction = match param_direction(param.attr.as_ref()) {
                HttpParamDirection::In => HttpSignatureParamDirection::In,
                HttpParamDirection::Out => HttpSignatureParamDirection::Out,
                HttpParamDirection::InOut => HttpSignatureParamDirection::InOut,
            };

            let deprecated = super::semantics::deprecated_info(&param.annotations)
                .map_err(|reason| format!("parameter '{}': {reason}", param.declarator.0))?;
            Ok(HttpSignatureParam {
                name: param.declarator.0.clone(),
                ty: param.ty.clone(),
                direction,
                is_optional: super::semantics::has_optional_annotation(&param.annotations),
                is_flatten: super::semantics::has_annotation(&param.annotations, "flatten"),
                annotations: get_signature_annotations(param)?,
                deprecated,
            })
        })
        .collect::<RestHirResult<_>>()?;

    Ok(HttpOperationSignature {
        params: sig_params,
        return_type: match &op.ty {
            hir::OpTypeSpec::Void => None,
            hir::OpTypeSpec::TypeSpec(ty) => Some(ty.clone()),
        },
    })
}

fn get_signature_annotations(
    param: &hir::ParamDcl,
) -> RestHirResult<Vec<HttpSignatureParamAnnotation>> {
    let mut sig_annos = Vec::new();
    if super::semantics::has_optional_annotation(&param.annotations) {
        sig_annos.push(HttpSignatureParamAnnotation::Optional);
    }
    if super::semantics::has_annotation(&param.annotations, "flatten") {
        sig_annos.push(HttpSignatureParamAnnotation::Flatten);
    }
    if super::semantics::has_annotation(&param.annotations, "body") {
        sig_annos.push(HttpSignatureParamAnnotation::Body);
    }

    if let Some(binding) =
        super::semantics::SourceBinding::read(&param.annotations, &param.declarator.0)?
    {
        let name = binding.wire_name(&param.declarator.0, &param.annotations, &[]);
        match binding.source {
            HttpParamKind::Path => sig_annos.push(HttpSignatureParamAnnotation::Path { name }),
            HttpParamKind::Query => sig_annos.push(HttpSignatureParamAnnotation::Query { name }),
            HttpParamKind::Header => sig_annos.push(HttpSignatureParamAnnotation::Header { name }),
            HttpParamKind::Cookie => sig_annos.push(HttpSignatureParamAnnotation::Cookie { name }),
            HttpParamKind::Body => {}
        }
    }
    Ok(sig_annos)
}

/// Returns true iff `ty` is `sequence<octet>`, indicating a raw byte buffer
/// that should be streamed without framing.
pub fn is_byte_sequence(ty: &hir::TypeSpec) -> bool {
    matches!(
        ty,
        hir::TypeSpec::SequenceType(seq)
            if matches!(*seq.ty, hir::TypeSpec::IntegerType(hir::IntegerType::Octet))
    )
}

pub fn build_http_mapping(
    method: HttpMethod,
    stream: &HttpStreamConfig,
    request_content_type: Option<&str>,
    response_content_type: Option<&str>,
    request_params: &[HttpParam],
    response_params: &[HttpParam],
    return_type: &Option<hir::TypeSpec>,
) -> HttpOperationHttpMapping {
    HttpOperationHttpMapping {
        request: build_request_mapping(stream, request_content_type, request_params),
        response: build_response_mapping(
            method,
            stream,
            response_content_type,
            response_params,
            return_type,
        ),
    }
}

fn build_request_mapping(
    stream: &HttpStreamConfig,
    content_type: Option<&str>,
    params: &[HttpParam],
) -> HttpRequestMapping {
    let mut path = Vec::new();
    let mut query = Vec::new();
    let mut header = Vec::new();
    let mut cookie = Vec::new();
    let mut body_params = Vec::new();

    for param in params {
        let binding = HttpInputBinding {
            source_param: param.name.clone(),
            wire_name: param.wire_name.clone(),
            ty: param.ty.clone(),
            optional: param.optional,
        };
        match param.kind {
            HttpParamKind::Path => path.push(binding),
            HttpParamKind::Query => query.push(binding),
            HttpParamKind::Header => header.push(binding),
            HttpParamKind::Cookie => cookie.push(binding),
            HttpParamKind::Body => body_params.push(param),
        }
    }

    let is_stream = matches!(
        stream.kind,
        Some(HttpStreamKind::Client | HttpStreamKind::Bidi)
    );
    let body_shape = if is_stream {
        // Stream
        if let Some(param) = body_params.first() {
            let codec = if body_params.len() == 1 && is_byte_sequence(&param.ty) {
                HttpStreamPayloadCodec::Bytes
            } else {
                map_stream_codec(stream.codec)
            };
            HttpRequestBodyShape::Stream {
                source_param: param.name.clone(),
                wire_name: param.wire_name.clone(),
                item_ty: param.ty.clone(),
                codec,
            }
        } else {
            HttpRequestBodyShape::Empty
        }
    } else if body_params.is_empty() {
        HttpRequestBodyShape::Empty
    } else if body_params.len() == 1 {
        HttpRequestBodyShape::SingleValue {
            source_param: body_params[0].name.clone(),
            wire_name: body_params[0].wire_name.clone(),
            flatten: body_params[0].flatten,
            ty: body_params[0].ty.clone(),
        }
    } else {
        HttpRequestBodyShape::Object {
            fields: body_params
                .iter()
                .map(|p| HttpRequestBodyField {
                    source_param: p.name.clone(),
                    field_name: p.wire_name.clone(),
                    ty: p.ty.clone(),
                    optional: p.optional,
                    flatten: p.flatten,
                })
                .collect(),
        }
    };

    let (final_content_type, final_codec) = if is_stream
        && !matches!(body_shape, HttpRequestBodyShape::Empty)
    {
        let ct = match &body_shape {
            HttpRequestBodyShape::Stream {
                codec: HttpStreamPayloadCodec::Bytes,
                ..
            } => "application/octet-stream",
            HttpRequestBodyShape::Stream {
                codec: HttpStreamPayloadCodec::Ndjson,
                ..
            } => "application/x-ndjson",
            HttpRequestBodyShape::Stream {
                codec: HttpStreamPayloadCodec::Sse,
                ..
            } => "text/event-stream",
            _ => unreachable!(),
        };
        (Some(ct.to_string()), map_body_codec(ct))
    } else if matches!(body_shape, HttpRequestBodyShape::Empty) {
        (None, None)
    } else if matches!(&body_shape, HttpRequestBodyShape::SingleValue { ty, .. } if is_byte_sequence(ty))
    {
        // A lone `sequence<octet>` body travels as raw bytes; senders may
        // still declare the payload's media type explicitly.
        (
            Some("application/octet-stream".to_string()),
            map_body_codec("application/octet-stream"),
        )
    } else {
        let ct = content_type.map(|s| s.to_string()).unwrap_or_else(|| {
            if is_composite_request_body(&body_shape) {
                "application/json".to_string()
            } else {
                "text/plain".to_string()
            }
        });
        (Some(ct.clone()), map_body_codec(&ct))
    };

    HttpRequestMapping {
        path,
        query,
        header,
        cookie,
        body: HttpRequestBodyMapping {
            content_type: final_content_type,
            content_type_explicit: content_type.is_some(),
            codec: final_codec,
            shape: body_shape,
        },
    }
}

fn build_response_mapping(
    method: HttpMethod,
    stream: &HttpStreamConfig,
    content_type: Option<&str>,
    params: &[HttpParam],
    return_type: &Option<hir::TypeSpec>,
) -> HttpResponseMapping {
    let mut header = Vec::new();
    let mut cookie = Vec::new();
    let mut body_outputs = Vec::new();

    for param in params {
        let binding = HttpOutputBinding {
            source: HttpOutputSource::Param {
                name: param.name.clone(),
            },
            wire_name: param.wire_name.clone(),
            ty: param.ty.clone(),
        };
        match param.kind {
            HttpParamKind::Header => header.push(binding),
            HttpParamKind::Cookie => cookie.push(binding),
            HttpParamKind::Body => body_outputs.push(param),
            _ => {}
        }
    }

    let is_stream = matches!(
        stream.kind,
        Some(HttpStreamKind::Server | HttpStreamKind::Bidi)
    );
    let body_shape = if is_stream {
        if let Some(ty) = return_type {
            let codec = if is_byte_sequence(ty) {
                HttpStreamPayloadCodec::Bytes
            } else {
                map_stream_codec(stream.codec)
            };
            HttpResponseBodyShape::Stream {
                item_source: HttpOutputSource::ReturnValue,
                item_ty: ty.clone(),
                codec,
            }
        } else if let Some(param) = body_outputs.first() {
            let codec = if body_outputs.len() == 1 && is_byte_sequence(&param.ty) {
                HttpStreamPayloadCodec::Bytes
            } else {
                map_stream_codec(stream.codec)
            };
            HttpResponseBodyShape::Stream {
                item_source: HttpOutputSource::Param {
                    name: param.name.clone(),
                },
                item_ty: param.ty.clone(),
                codec,
            }
        } else {
            HttpResponseBodyShape::Empty
        }
    } else if return_type.is_none() && body_outputs.is_empty() {
        HttpResponseBodyShape::Empty
    } else if return_type.is_some() && body_outputs.is_empty() {
        HttpResponseBodyShape::ReturnOnly {
            ty: return_type.as_ref().unwrap().clone(),
        }
    } else if return_type.is_none() && body_outputs.len() == 1 {
        HttpResponseBodyShape::SingleValue {
            source: HttpOutputSource::Param {
                name: body_outputs[0].name.clone(),
            },
            ty: body_outputs[0].ty.clone(),
        }
    } else {
        let mut fields = Vec::new();
        if let Some(ty) = return_type {
            fields.push(HttpResponseBodyField {
                source: HttpOutputSource::ReturnValue,
                field_name: "return".to_string(),
                ty: ty.clone(),
            });
        }
        for p in body_outputs {
            fields.push(HttpResponseBodyField {
                source: HttpOutputSource::Param {
                    name: p.name.clone(),
                },
                field_name: p.wire_name.clone(),
                ty: p.ty.clone(),
            });
        }
        HttpResponseBodyShape::Object { fields }
    };

    let (final_content_type, final_codec) =
        if is_stream && !matches!(body_shape, HttpResponseBodyShape::Empty) {
            let ct = match &body_shape {
                HttpResponseBodyShape::Stream {
                    codec: HttpStreamPayloadCodec::Bytes,
                    ..
                } => "application/octet-stream",
                HttpResponseBodyShape::Stream {
                    codec: HttpStreamPayloadCodec::Ndjson,
                    ..
                } => "application/x-ndjson",
                HttpResponseBodyShape::Stream {
                    codec: HttpStreamPayloadCodec::Sse,
                    ..
                } => "text/event-stream",
                _ => unreachable!(),
            };
            (Some(ct.to_string()), map_body_codec(ct))
        } else if matches!(body_shape, HttpResponseBodyShape::Empty) {
            (None, None)
        } else {
            let ct = content_type.map(|s| s.to_string()).unwrap_or_else(|| {
                if is_composite_response_body(&body_shape) {
                    "application/json".to_string()
                } else {
                    "text/plain".to_string()
                }
            });
            (Some(ct.clone()), map_body_codec(&ct))
        };

    let status = if matches!(method, HttpMethod::Head) {
        "200".to_string()
    } else if matches!(body_shape, HttpResponseBodyShape::Empty) {
        "204".to_string()
    } else {
        "200".to_string()
    };

    HttpResponseMapping {
        representations: Vec::new(),
        header,
        cookie,
        body: HttpResponseBodyMapping {
            content_type: final_content_type,
            content_type_explicit: content_type.is_some(),
            codec: final_codec,
            shape: body_shape,
        },
        status,
    }
}
