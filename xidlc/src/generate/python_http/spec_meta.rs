use crate::error::{IdlcError, IdlcResult};
use crate::generate::http_hir::{
    HttpMethod, HttpOperation, HttpParamSource,
    semantics::{
        HttpApiKeyLocation, HttpSecurityProfile, HttpSecurityRequirement, HttpStreamCodec,
        HttpStreamConfig, HttpStreamKind,
    },
};

use super::spec_context::ParamSource;

pub(super) fn validate_stream_support(operation: &HttpOperation) -> IdlcResult<()> {
    let stream_kind = operation.stream.kind;
    let stream_codec = operation.stream.codec;
    if matches!(stream_kind, Some(HttpStreamKind::Bidi)) {
        return Err(IdlcError::rpc(format!(
            "python-http currently does not support @bidi_stream methods: '{}'",
            operation.name
        )));
    }
    if matches!(stream_kind, Some(HttpStreamKind::Server)) && stream_codec != HttpStreamCodec::Sse {
        return Err(IdlcError::rpc(format!(
            "python-http currently supports only SSE for @server_stream methods: '{}'",
            operation.name
        )));
    }
    if matches!(stream_kind, Some(HttpStreamKind::Client))
        && stream_codec != HttpStreamCodec::Ndjson
    {
        return Err(IdlcError::rpc(format!(
            "python-http currently supports only NDJSON for @client_stream methods: '{}'",
            operation.name
        )));
    }
    Ok(())
}

pub(super) fn request_content_type(operation: &HttpOperation) -> String {
    match operation.stream.kind {
        Some(HttpStreamKind::Client) => "application/x-ndjson".to_string(),
        _ => operation.request_content_type.clone(),
    }
}

pub(super) fn response_content_type(operation: &HttpOperation) -> String {
    match (operation.stream.kind, operation.stream.codec) {
        (Some(HttpStreamKind::Server), HttpStreamCodec::Sse) => "text/event-stream".to_string(),
        _ => operation.response_content_type.clone(),
    }
}

pub(super) fn security_expr(value: Option<&HttpSecurityProfile>) -> String {
    let Some(value) = value else {
        return "[]".to_string();
    };
    if value.requirements.is_empty() {
        return "[SecurityRequirement(kind=\"none\")]".to_string();
    }
    let parts = value
        .requirements
        .iter()
        .map(|requirement| match requirement {
            HttpSecurityRequirement::HttpBasic => "SecurityRequirement(kind=\"basic\")".to_string(),
            HttpSecurityRequirement::HttpBearer => {
                "SecurityRequirement(kind=\"bearer\")".to_string()
            }
            HttpSecurityRequirement::ApiKey { location, name } => format!(
                "SecurityRequirement(kind=\"api_key\", name={:?}, location={:?})",
                name,
                api_key_location(location)
            ),
            HttpSecurityRequirement::OAuth2 { scopes } => {
                format!("SecurityRequirement(kind=\"oauth2\", scopes={:?})", scopes)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{parts}]")
}

pub(super) fn stream_expr(value: HttpStreamConfig) -> String {
    match value.kind {
        None => "None".to_string(),
        Some(kind) => format!(
            "StreamMetadata(kind={:?}, codec={:?})",
            match kind {
                HttpStreamKind::Server => "server",
                HttpStreamKind::Client => "client",
                HttpStreamKind::Bidi => "bidi",
            },
            stream_codec_name(value.codec)
        ),
    }
}

pub(super) fn http_method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

pub(super) fn param_source(source: HttpParamSource) -> ParamSource {
    match source {
        HttpParamSource::Path => ParamSource::Path,
        HttpParamSource::Query => ParamSource::Query,
        HttpParamSource::Header => ParamSource::Header,
        HttpParamSource::Cookie => ParamSource::Cookie,
        HttpParamSource::Body => ParamSource::Body,
    }
}

fn stream_codec_name(value: HttpStreamCodec) -> &'static str {
    match value {
        HttpStreamCodec::Sse => "sse",
        HttpStreamCodec::Ndjson => "ndjson",
    }
}

fn api_key_location(value: &HttpApiKeyLocation) -> &'static str {
    match value {
        HttpApiKeyLocation::Header => "header",
        HttpApiKeyLocation::Query => "query",
        HttpApiKeyLocation::Cookie => "cookie",
    }
}
