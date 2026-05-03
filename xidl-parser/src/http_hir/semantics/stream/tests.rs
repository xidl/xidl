use super::*;
use crate::hir::{Annotation, AnnotationParams};

fn builtin(name: &str, raw: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: Some(AnnotationParams::Raw(raw.to_string())),
    }
}

fn bare(name: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: None,
    }
}

fn support() -> HttpStreamTargetSupport<'static> {
    HttpStreamTargetSupport {
        target: "axum",
        supports_bidi: false,
        server_codec: HttpStreamCodec::Sse,
        client_codec: HttpStreamCodec::Ndjson,
        server_method: "GET",
        client_method: "POST",
        bidi_method: "GET",
    }
}

#[test]
fn http_stream_config_infers_defaults_and_validates_annotations() {
    assert_eq!(
        http_stream_config(&[bare("server_stream")]).expect("server config"),
        HttpStreamConfig {
            kind: Some(HttpStreamKind::Server),
            codec: HttpStreamCodec::Sse,
        }
    );
    assert_eq!(
        http_stream_config(&[bare("client_stream")]).expect("client config"),
        HttpStreamConfig {
            kind: Some(HttpStreamKind::Client),
            codec: HttpStreamCodec::Ndjson,
        }
    );
    assert_eq!(
        http_stream_config(&[builtin("stream_codec", "\"ndjson\"")]).expect("codec only"),
        HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        }
    );

    let invalid =
        http_stream_config(&[builtin("stream_codec", "\"xml\"")]).expect_err("invalid codec");
    assert!(invalid.contains("unsupported @stream_codec value"));

    let conflict =
        http_stream_config(&[bare("server_stream"), bare("client_stream")]).expect_err("conflict");
    assert!(conflict.contains("mutually exclusive"));
}

#[test]
fn validate_http_stream_target_reports_target_capability_errors() {
    let support = support();
    assert!(
        validate_http_stream_target(
            "watch",
            HttpStreamConfig {
                kind: Some(HttpStreamKind::Server),
                codec: HttpStreamCodec::Ndjson,
            },
            support,
        )
        .expect_err("server codec")
        .contains("supports only SSE")
    );
    assert!(
        validate_http_stream_target(
            "upload",
            HttpStreamConfig {
                kind: Some(HttpStreamKind::Client),
                codec: HttpStreamCodec::Sse,
            },
            support,
        )
        .expect_err("client sse")
        .contains("supports only NDJSON")
    );
    let mut bidi_support = support;
    bidi_support.supports_bidi = true;
    bidi_support.client_codec = HttpStreamCodec::Sse;
    assert!(
        validate_http_stream_target(
            "upload",
            HttpStreamConfig {
                kind: Some(HttpStreamKind::Client),
                codec: HttpStreamCodec::Sse,
            },
            bidi_support,
        )
        .expect_err("client sse only for server")
        .contains("requires @server_stream")
    );
    assert!(
        validate_http_stream_target(
            "chat",
            HttpStreamConfig {
                kind: Some(HttpStreamKind::Bidi),
                codec: HttpStreamCodec::Ndjson,
            },
            support,
        )
        .expect_err("unsupported bidi")
        .contains("does not support @bidi_stream")
    );
    assert!(
        validate_http_stream_target(
            "import",
            HttpStreamConfig {
                kind: Some(HttpStreamKind::Client),
                codec: HttpStreamCodec::Ndjson,
            },
            support,
        )
        .is_ok()
    );
}

#[test]
fn validate_http_stream_method_enforces_expected_http_verb() {
    let support = support();
    assert!(
        validate_http_stream_method("watch", Some(HttpStreamKind::Server), "POST", support)
            .expect_err("server method")
            .contains("@server_stream method 'watch' must use GET")
    );
    assert!(
        validate_http_stream_method("upload", Some(HttpStreamKind::Client), "GET", support)
            .expect_err("client method")
            .contains("@client_stream method 'upload' must use POST")
    );
    assert!(
        validate_http_stream_method("chat", Some(HttpStreamKind::Bidi), "POST", support)
            .expect_err("bidi method")
            .contains("@bidi_stream method 'chat' must use GET")
    );
    assert!(validate_http_stream_method("watch", None, "PATCH", support).is_ok());
}
