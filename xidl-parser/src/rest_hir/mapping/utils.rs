use super::*;
use crate::hir;
use crate::rest_hir::semantics::HttpStreamCodec;

pub(crate) fn is_composite_request_body(shape: &HttpRequestBodyShape) -> bool {
    match shape {
        HttpRequestBodyShape::Empty => false,
        HttpRequestBodyShape::SingleValue { ty, .. } => is_composite_type(ty),
        HttpRequestBodyShape::Object { .. } => true,
        HttpRequestBodyShape::Stream { .. } => true,
    }
}

pub(crate) fn is_composite_response_body(shape: &HttpResponseBodyShape) -> bool {
    match shape {
        HttpResponseBodyShape::Empty => false,
        HttpResponseBodyShape::ReturnOnly { ty } => is_composite_type(ty),
        HttpResponseBodyShape::SingleValue { ty, .. } => is_composite_type(ty),
        HttpResponseBodyShape::Object { .. } => true,
        HttpResponseBodyShape::Stream { .. } => true,
    }
}

pub(crate) fn is_composite_type(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::IntegerType(_)
        | hir::TypeSpec::FloatingPtType
        | hir::TypeSpec::CharType
        | hir::TypeSpec::WideCharType
        | hir::TypeSpec::Boolean
        | hir::TypeSpec::StringType(_)
        | hir::TypeSpec::WideStringType(_)
        | hir::TypeSpec::FixedPtType(_) => false,
        hir::TypeSpec::ScopedName(_)
        | hir::TypeSpec::SequenceType(_)
        | hir::TypeSpec::MapType(_)
        | hir::TypeSpec::TemplateType(_)
        | hir::TypeSpec::AnyType
        | hir::TypeSpec::ObjectType
        | hir::TypeSpec::ValueBaseType => true,
    }
}

pub(crate) fn map_body_codec(content_type: &str) -> Option<HttpBodyCodec> {
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

pub(crate) fn map_stream_codec(codec: HttpStreamCodec) -> HttpStreamPayloadCodec {
    match codec {
        HttpStreamCodec::Ndjson => HttpStreamPayloadCodec::Ndjson,
        HttpStreamCodec::Sse => HttpStreamPayloadCodec::Sse,
    }
}
