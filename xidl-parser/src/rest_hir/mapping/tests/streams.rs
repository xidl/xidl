use super::*;
use crate::rest_hir::semantics::HttpStreamCodec;

#[test]
fn test_build_http_mapping_covers_bindings_streams_and_codecs() {
    let client_stream = HttpStreamConfig {
        kind: Some(HttpStreamKind::Client),
        codec: HttpStreamCodec::Ndjson,
    };
    let mapping = build_http_mapping(
        HttpMethod::Post,
        &client_stream,
        Some("application/msgpack"),
        Some("application/x-custom"),
        &[
            string_param("id", HttpParamKind::Path),
            string_param("q", HttpParamKind::Query),
            string_param("trace", HttpParamKind::Header),
            string_param("session", HttpParamKind::Cookie),
            string_param("item", HttpParamKind::Body),
        ],
        &[
            string_param("etag", HttpParamKind::Header),
            string_param("token", HttpParamKind::Cookie),
            string_param("ignored", HttpParamKind::Query),
        ],
        &None,
    );

    assert_eq!(mapping.request.path[0].source_param, "id");
    assert_eq!(mapping.request.query[0].source_param, "q");
    assert_eq!(mapping.request.header[0].source_param, "trace");
    assert_eq!(mapping.request.cookie[0].source_param, "session");
    assert_eq!(
        mapping.request.body.content_type.as_deref(),
        Some("application/x-ndjson")
    );
    assert_eq!(mapping.request.body.codec, None);
    assert!(matches!(
        mapping.request.body.shape,
        HttpRequestBodyShape::Stream {
            codec: HttpStreamPayloadCodec::Ndjson,
            ..
        }
    ));
    assert_eq!(mapping.response.header[0].wire_name, "etag");
    assert_eq!(mapping.response.cookie[0].wire_name, "token");
    assert_eq!(mapping.response.body.content_type, None);

    let server_stream = HttpStreamConfig {
        kind: Some(HttpStreamKind::Server),
        codec: HttpStreamCodec::Sse,
    };
    let mapping = build_http_mapping(
        HttpMethod::Get,
        &server_stream,
        Some("application/x-www-form-urlencoded"),
        Some("application/msgpack"),
        &[],
        &[string_param("events", HttpParamKind::Body)],
        &None,
    );
    assert_eq!(
        mapping.response.body.content_type.as_deref(),
        Some("text/event-stream")
    );
    assert!(matches!(
        mapping.response.body.shape,
        HttpResponseBodyShape::Stream {
            item_source: HttpOutputSource::Param { .. },
            codec: HttpStreamPayloadCodec::Sse,
            ..
        }
    ));

    let explicit = build_http_mapping(
        HttpMethod::Head,
        &HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        },
        Some("application/x-www-form-urlencoded"),
        Some("application/msgpack"),
        &[string_param("form", HttpParamKind::Body)],
        &[string_param("value", HttpParamKind::Body)],
        &None,
    );
    assert_eq!(
        explicit.request.body.codec,
        Some(HttpBodyCodec::FormUrlEncoded)
    );
    assert_eq!(explicit.response.body.codec, Some(HttpBodyCodec::Msgpack));
    assert_eq!(explicit.response.status, "204");
}

#[test]
fn test_build_http_mapping_covers_empty_stream_and_single_output_body() {
    let client_stream_without_body = build_http_mapping(
        HttpMethod::Post,
        &HttpStreamConfig {
            kind: Some(HttpStreamKind::Client),
            codec: HttpStreamCodec::Sse,
        },
        None,
        None,
        &[],
        &[],
        &None,
    );
    assert!(matches!(
        client_stream_without_body.request.body.shape,
        HttpRequestBodyShape::Empty
    ));
    assert_eq!(client_stream_without_body.request.body.content_type, None);

    let server_stream_without_body = build_http_mapping(
        HttpMethod::Get,
        &HttpStreamConfig {
            kind: Some(HttpStreamKind::Server),
            codec: HttpStreamCodec::Ndjson,
        },
        None,
        None,
        &[],
        &[],
        &None,
    );
    assert!(matches!(
        server_stream_without_body.response.body.shape,
        HttpResponseBodyShape::Empty
    ));
    assert_eq!(server_stream_without_body.response.status, "204");

    let single_output = build_http_mapping(
        HttpMethod::Post,
        &HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        },
        None,
        None,
        &[],
        &[string_param("value", HttpParamKind::Body)],
        &None,
    );
    assert!(matches!(
        single_output.response.body.shape,
        HttpResponseBodyShape::SingleValue {
            source: HttpOutputSource::Param { .. },
            ..
        }
    ));
    assert_eq!(
        single_output.response.body.content_type.as_deref(),
        Some("text/plain")
    );
    assert_eq!(single_output.response.status, "200");
}
