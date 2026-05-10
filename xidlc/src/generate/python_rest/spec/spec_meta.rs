use crate::error::{IdlcError, IdlcResult};
use xidl_parser::rest_hir::{
    HttpMethod, HttpOperation,
    semantics::{
        HttpApiKeyLocation, HttpSecurityProfile, HttpSecurityRequirement, HttpStreamCodec,
        HttpStreamKind,
    },
};

pub(super) fn validate_stream_support(operation: &HttpOperation) -> IdlcResult<()> {
    let stream_kind = operation.meta.stream.kind;
    let stream_codec = operation.meta.stream.codec;
    if matches!(stream_kind, Some(HttpStreamKind::Bidi)) {
        return Err(IdlcError::rpc(format!(
            "python-rest currently does not support @bidi_stream methods: '{}'",
            operation.meta.name
        )));
    }
    if matches!(stream_kind, Some(HttpStreamKind::Server)) && stream_codec != HttpStreamCodec::Sse {
        return Err(IdlcError::rpc(format!(
            "python-rest currently supports only SSE for @server_stream methods: '{}'",
            operation.meta.name
        )));
    }
    if matches!(stream_kind, Some(HttpStreamKind::Client))
        && stream_codec != HttpStreamCodec::Ndjson
    {
        return Err(IdlcError::rpc(format!(
            "python-rest currently supports only NDJSON for @client_stream methods: '{}'",
            operation.meta.name
        )));
    }
    Ok(())
}

pub(super) fn security_expr(
    value: Option<&HttpSecurityProfile>,
    basic_auth_realm: Option<&String>,
) -> String {
    let Some(value) = value else {
        return "[]".to_string();
    };
    if value.requirements.is_empty() {
        return "[]".to_string();
    }
    let mut expr = String::from("[");
    for req in &value.requirements {
        match req {
            HttpSecurityRequirement::HttpBasic => {
                expr.push_str(&format!(
                    "SecurityRequirement(kind=\"basic\", realm={:?}), ",
                    basic_auth_realm
                ));
            }
            HttpSecurityRequirement::HttpBearer => {
                expr.push_str("SecurityRequirement(kind=\"bearer\"), ");
            }
            HttpSecurityRequirement::ApiKey { location, name } => {
                expr.push_str(&format!(
                    "SecurityRequirement(kind=\"api_key\", location={:?}, name={:?}), ",
                    api_key_location(location),
                    name
                ));
            }
            HttpSecurityRequirement::OAuth2 { scopes } => {
                expr.push_str(&format!(
                    "SecurityRequirement(kind=\"oauth2\", scopes={:?}), ",
                    scopes
                ));
            }
        }
    }
    expr.push(']');
    expr
}

pub(super) fn stream_expr(value: xidl_parser::rest_hir::semantics::HttpStreamConfig) -> String {
    if let Some(kind) = value.kind {
        format!(
            "StreamMetadata(kind={:?}, codec={:?})",
            format!("{kind:?}").to_lowercase(),
            stream_codec_name(value.codec)
        )
    } else {
        "None".to_string()
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

fn api_key_location(value: &HttpApiKeyLocation) -> &'static str {
    match value {
        HttpApiKeyLocation::Header => "header",
        HttpApiKeyLocation::Query => "query",
        HttpApiKeyLocation::Cookie => "cookie",
    }
}

fn stream_codec_name(value: HttpStreamCodec) -> &'static str {
    match value {
        HttpStreamCodec::Sse => "sse",
        HttpStreamCodec::Ndjson => "ndjson",
    }
}
