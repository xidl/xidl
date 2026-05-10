use super::semantics::{HttpStreamCodec, HttpStreamConfig, HttpStreamKind};
use super::validate::{HttpParamDirection, param_direction};
use super::*;
use crate::hir;

#[cfg(test)]
mod tests;

pub fn build_operation_signature(op: &hir::OpDcl) -> HttpOperationSignature {
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

            HttpSignatureParam {
                name: param.declarator.0.clone(),
                ty: param.ty.clone(),
                direction,
                is_optional: super::semantics::has_optional_annotation(&param.annotations),
                is_flatten: super::semantics::has_annotation(&param.annotations, "flatten"),
                annotations: get_signature_annotations(param),
            }
        })
        .collect();

    HttpOperationSignature {
        params: sig_params,
        return_type: match &op.ty {
            hir::OpTypeSpec::Void => None,
            hir::OpTypeSpec::TypeSpec(ty) => Some(ty.clone()),
        },
    }
}

fn get_signature_annotations(param: &hir::ParamDcl) -> Vec<HttpSignatureParamAnnotation> {
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

    for annotation in &param.annotations {
        let Some(name) = super::semantics::annotation_name(annotation) else {
            continue;
        };
        let name_lc = name.to_lowercase();
        let value = super::semantics::annotation_params(annotation)
            .map(super::semantics::normalize_annotation_params)
            .and_then(|p| p.get("value").cloned())
            .unwrap_or_else(|| param.declarator.0.clone());

        match name_lc.as_str() {
            "path" => sig_annos.push(HttpSignatureParamAnnotation::Path { name: value }),
            "query" => sig_annos.push(HttpSignatureParamAnnotation::Query { name: value }),
            "header" => sig_annos.push(HttpSignatureParamAnnotation::Header { name: value }),
            "cookie" => sig_annos.push(HttpSignatureParamAnnotation::Cookie { name: value }),
            _ => {}
        }
    }
    sig_annos
}

pub fn build_http_mapping(
    method: HttpMethod,
    stream: &HttpStreamConfig,
    request_content_type: &str,
    response_content_type: &str,
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
    content_type: &str,
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
            HttpRequestBodyShape::Stream {
                source_param: param.name.clone(),
                item_ty: param.ty.clone(),
                codec: map_stream_codec(stream.codec),
            }
        } else {
            HttpRequestBodyShape::Empty
        }
    } else if body_params.is_empty() {
        HttpRequestBodyShape::Empty
    } else if body_params.len() == 1 {
        HttpRequestBodyShape::SingleValue {
            source_param: body_params[0].name.clone(),
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

    let (final_content_type, final_codec) =
        if is_stream && !matches!(body_shape, HttpRequestBodyShape::Empty) {
            let ct = match stream.codec {
                HttpStreamCodec::Ndjson => "application/x-ndjson",
                HttpStreamCodec::Sse => "text/event-stream",
            };
            (Some(ct.to_string()), map_body_codec(ct))
        } else if matches!(body_shape, HttpRequestBodyShape::Empty) {
            (None, None)
        } else {
            (Some(content_type.to_string()), map_body_codec(content_type))
        };

    HttpRequestMapping {
        path,
        query,
        header,
        cookie,
        body: HttpRequestBodyMapping {
            content_type: final_content_type,
            codec: final_codec,
            shape: body_shape,
        },
    }
}

fn build_response_mapping(
    method: HttpMethod,
    stream: &HttpStreamConfig,
    content_type: &str,
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
            HttpResponseBodyShape::Stream {
                item_source: HttpOutputSource::ReturnValue,
                item_ty: ty.clone(),
                codec: map_stream_codec(stream.codec),
            }
        } else if let Some(param) = body_outputs.first() {
            HttpResponseBodyShape::Stream {
                item_source: HttpOutputSource::Param {
                    name: param.name.clone(),
                },
                item_ty: param.ty.clone(),
                codec: map_stream_codec(stream.codec),
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
            let ct = match stream.codec {
                HttpStreamCodec::Ndjson => "application/x-ndjson",
                HttpStreamCodec::Sse => "text/event-stream",
            };
            (Some(ct.to_string()), map_body_codec(ct))
        } else if matches!(body_shape, HttpResponseBodyShape::Empty) {
            (None, None)
        } else {
            (Some(content_type.to_string()), map_body_codec(content_type))
        };

    let status = if matches!(method, HttpMethod::Head)
        || matches!(body_shape, HttpResponseBodyShape::Empty)
    {
        "204".to_string()
    } else {
        "200".to_string()
    };

    HttpResponseMapping {
        header,
        cookie,
        body: HttpResponseBodyMapping {
            content_type: final_content_type,
            codec: final_codec,
            shape: body_shape,
        },
        status,
    }
}

fn map_body_codec(content_type: &str) -> Option<HttpBodyCodec> {
    if content_type.eq_ignore_ascii_case("application/json") {
        Some(HttpBodyCodec::Json)
    } else if content_type.eq_ignore_ascii_case("application/x-www-form-urlencoded") {
        Some(HttpBodyCodec::FormUrlEncoded)
    } else if content_type.eq_ignore_ascii_case("application/msgpack") {
        Some(HttpBodyCodec::Msgpack)
    } else if content_type.eq_ignore_ascii_case("text/plain") {
        Some(HttpBodyCodec::Text)
    } else {
        None
    }
}

fn map_stream_codec(codec: HttpStreamCodec) -> HttpStreamPayloadCodec {
    match codec {
        HttpStreamCodec::Ndjson => HttpStreamPayloadCodec::Ndjson,
        HttpStreamCodec::Sse => HttpStreamPayloadCodec::Sse,
    }
}
