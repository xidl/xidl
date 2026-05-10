use crate::error::{IdlcError, IdlcResult};
use crate::generate::go_rest::HttpMethod;
use xidl_parser::rest_hir::{
    HttpMethod as RestHirMethod, HttpOperation,
    semantics::{HttpStreamCodec, HttpStreamKind},
};

pub(super) fn validate_stream_support(op: &HttpOperation) -> IdlcResult<()> {
    match op.meta.stream.kind {
        Some(HttpStreamKind::Server) if op.meta.stream.codec != HttpStreamCodec::Sse => {
            Err(IdlcError::rpc(format!(
                "go-rest currently supports only SSE for @server_stream methods: '{}'",
                op.meta.name
            )))
        }
        Some(HttpStreamKind::Client) if op.meta.stream.codec != HttpStreamCodec::Ndjson => {
            Err(IdlcError::rpc(format!(
                "go-rest currently supports only NDJSON for @client_stream methods: '{}'",
                op.meta.name
            )))
        }
        Some(HttpStreamKind::Bidi) => Err(IdlcError::rpc(format!(
            "go-rest currently does not support @bidi_stream methods: '{}'",
            op.meta.name
        ))),
        _ => Ok(()),
    }
}

pub(super) struct DeprecatedContext {
    pub(super) deprecated: bool,
    pub(super) since: Option<String>,
    pub(super) after: Option<String>,
    pub(super) note: Option<String>,
}

pub(super) fn deprecated_context(op: &HttpOperation) -> DeprecatedContext {
    let info = op.meta.deprecated.as_ref();
    let deprecated = info.as_ref().map(|value| value.deprecated).unwrap_or(false);
    let since = info.as_ref().and_then(|value| value.since.clone());
    let after = info.as_ref().and_then(|value| value.after.clone());
    let note = info.as_ref().map(|value| {
        let mut note = String::from("Deprecated.");
        if let Some(since) = &value.since {
            note.push_str(&format!(" Since {since}."));
        }
        if let Some(after) = &value.after {
            note.push_str(&format!(" After {after}."));
        }
        note
    });
    DeprecatedContext {
        deprecated,
        since,
        after,
        note,
    }
}

pub(super) fn http_method(method: RestHirMethod) -> HttpMethod {
    match method {
        RestHirMethod::Get => HttpMethod::Get,
        RestHirMethod::Post => HttpMethod::Post,
        RestHirMethod::Put => HttpMethod::Put,
        RestHirMethod::Patch => HttpMethod::Patch,
        RestHirMethod::Delete => HttpMethod::Delete,
        RestHirMethod::Head => HttpMethod::Head,
        RestHirMethod::Options => HttpMethod::Options,
    }
}
