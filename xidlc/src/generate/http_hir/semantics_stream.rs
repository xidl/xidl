use serde::{Deserialize, Serialize};
use xidl_parser::hir;

use super::semantics_annotations::{
    annotation_name, annotation_params, normalize_annotation_params,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpStreamKind {
    Server,
    Client,
    Bidi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpStreamCodec {
    Sse,
    Ndjson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpStreamConfig {
    pub kind: Option<HttpStreamKind>,
    pub codec: HttpStreamCodec,
}

#[derive(Debug, Clone, Copy)]
pub struct HttpStreamTargetSupport<'a> {
    pub target: &'a str,
    pub supports_bidi: bool,
    pub server_codec: HttpStreamCodec,
    pub client_codec: HttpStreamCodec,
    pub server_method: &'a str,
    pub client_method: &'a str,
    pub bidi_method: &'a str,
}

pub fn http_stream_config(annotations: &[hir::Annotation]) -> Result<HttpStreamConfig, String> {
    let kind = stream_kind(annotations)?;
    let mut codec = match kind {
        Some(HttpStreamKind::Server) => HttpStreamCodec::Sse,
        Some(HttpStreamKind::Client | HttpStreamKind::Bidi) | None => HttpStreamCodec::Ndjson,
    };
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("stream_codec") {
            continue;
        }
        let value = annotation_params(annotation)
            .map(normalize_annotation_params)
            .and_then(|params| params.get("value").cloned())
            .unwrap_or_else(|| "sse".to_string());
        codec = match value.to_ascii_lowercase().as_str() {
            "sse" => HttpStreamCodec::Sse,
            "ndjson" => HttpStreamCodec::Ndjson,
            other => {
                return Err(format!(
                    "unsupported @stream_codec value '{other}', expected 'sse' or 'ndjson'"
                ));
            }
        };
    }
    Ok(HttpStreamConfig { kind, codec })
}

pub fn validate_http_stream_target(
    op_name: &str,
    config: HttpStreamConfig,
    support: HttpStreamTargetSupport<'_>,
) -> Result<(), String> {
    match config.kind {
        Some(HttpStreamKind::Server) if config.codec != support.server_codec => Err(format!(
            "{} currently supports only {} for @server_stream methods: '{}'",
            support.target,
            stream_codec_name(support.server_codec),
            op_name
        )),
        Some(HttpStreamKind::Client) if config.codec != support.client_codec => Err(format!(
            "{} currently supports only {} for @client_stream methods: '{}'",
            support.target,
            stream_codec_name(support.client_codec),
            op_name
        )),
        Some(HttpStreamKind::Bidi) if !support.supports_bidi => Err(format!(
            "{} currently does not support @bidi_stream methods: '{}'",
            support.target, op_name
        )),
        Some(HttpStreamKind::Client | HttpStreamKind::Bidi)
            if config.codec == HttpStreamCodec::Sse =>
        {
            Err(format!(
                "@stream_codec(\"sse\") requires @server_stream on method '{}'",
                op_name
            ))
        }
        _ => Ok(()),
    }
}

pub fn validate_http_stream_method(
    op_name: &str,
    kind: Option<HttpStreamKind>,
    method: &str,
    support: HttpStreamTargetSupport<'_>,
) -> Result<(), String> {
    let method = method.to_ascii_uppercase();
    match kind {
        Some(HttpStreamKind::Server) if method != support.server_method => Err(format!(
            "@server_stream method '{}' must use {}",
            op_name, support.server_method
        )),
        Some(HttpStreamKind::Client) if method != support.client_method => Err(format!(
            "@client_stream method '{}' must use {}",
            op_name, support.client_method
        )),
        Some(HttpStreamKind::Bidi) if method != support.bidi_method => Err(format!(
            "@bidi_stream method '{}' must use {}",
            op_name, support.bidi_method
        )),
        _ => Ok(()),
    }
}

fn stream_kind(annotations: &[hir::Annotation]) -> Result<Option<HttpStreamKind>, String> {
    let mut kind = None;
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("server_stream") {
            Some(HttpStreamKind::Server)
        } else if name.eq_ignore_ascii_case("client_stream") {
            Some(HttpStreamKind::Client)
        } else if name.eq_ignore_ascii_case("bidi_stream") {
            Some(HttpStreamKind::Bidi)
        } else {
            None
        };
        let Some(current) = current else {
            continue;
        };
        match kind {
            None => kind = Some(current),
            Some(prev) if prev == current => {}
            Some(_) => {
                return Err(
                    "@server_stream/@client_stream/@bidi_stream are mutually exclusive".to_string(),
                );
            }
        }
    }
    Ok(kind)
}

fn stream_codec_name(codec: HttpStreamCodec) -> &'static str {
    match codec {
        HttpStreamCodec::Sse => "SSE",
        HttpStreamCodec::Ndjson => "NDJSON",
    }
}
