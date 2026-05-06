use crate::error::{IdlcError, IdlcResult};
use crate::generate::go_rest::{HttpMethod, ParamMeta, ParamSource, definition};
use convert_case::{Case, Casing};
use xidl_parser::rest_hir::{
    HttpMethod as RestHirMethod, HttpOperation, HttpParamKind as RestHirParamKind,
    semantics::{HttpStreamCodec, HttpStreamKind},
};

pub(super) fn validate_stream_support(op: &HttpOperation) -> IdlcResult<()> {
    match op.stream.kind {
        Some(HttpStreamKind::Server) if op.stream.codec != HttpStreamCodec::Sse => {
            Err(IdlcError::rpc(format!(
                "go-rest currently supports only SSE for @server_stream methods: '{}'",
                op.name
            )))
        }
        Some(HttpStreamKind::Client) if op.stream.codec != HttpStreamCodec::Ndjson => {
            Err(IdlcError::rpc(format!(
                "go-rest currently supports only NDJSON for @client_stream methods: '{}'",
                op.name
            )))
        }
        Some(HttpStreamKind::Bidi) => Err(IdlcError::rpc(format!(
            "go-rest currently does not support @bidi_stream methods: '{}'",
            op.name
        ))),
        _ => Ok(()),
    }
}

pub(super) fn request_body_context(
    body_params: &[ParamMeta],
    struct_prefix: &str,
    stream_kind: Option<HttpStreamKind>,
) -> (Option<String>, Option<String>, Option<String>) {
    if body_params.is_empty() || matches!(stream_kind, Some(HttpStreamKind::Client)) {
        (None, None, None)
    } else if body_params.len() == 1 && body_params[0].flatten {
        let param = &body_params[0];
        (None, Some(param.field_name.clone()), Some(param.ty.clone()))
    } else {
        (Some(format!("{struct_prefix}RequestBody")), None, None)
    }
}

pub(super) fn response_body_context(
    response_body_params: &[ParamMeta],
    return_ty: &Option<String>,
    struct_prefix: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    let response_output_count = response_body_params.len() + usize::from(return_ty.is_some());
    if response_output_count == 0 {
        (None, None, None)
    } else if response_output_count == 1 {
        if let Some(return_ty) = return_ty {
            (None, Some("Return".to_string()), Some(return_ty.clone()))
        } else {
            (Some(format!("{struct_prefix}ResponseBody")), None, None)
        }
    } else {
        (Some(format!("{struct_prefix}ResponseBody")), None, None)
    }
}

pub(super) fn response_params_from_meta(
    response_body_params: &mut Vec<ParamMeta>,
    response_header_params: &mut Vec<ParamMeta>,
    response_cookie_params: &mut Vec<ParamMeta>,
    meta: ParamMeta,
) {
    match meta.source {
        ParamSource::Header => response_header_params.push(meta),
        ParamSource::Cookie => response_cookie_params.push(meta),
        _ => response_body_params.push(meta),
    }
}

pub(super) struct DeprecatedContext {
    pub(super) deprecated: bool,
    pub(super) since: Option<String>,
    pub(super) after: Option<String>,
    pub(super) note: Option<String>,
}

pub(super) fn deprecated_context(op: &HttpOperation) -> DeprecatedContext {
    let info = op.deprecated.as_ref();
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

pub(super) fn param_meta(param: &xidl_parser::rest_hir::HttpParam) -> ParamMeta {
    ParamMeta {
        field_name: param.name.to_case(Case::Pascal),
        raw_name: param.name.clone(),
        wire_name: param.wire_name.clone(),
        ty: definition::go_type(&param.ty),
        optional: param.optional,
        source: match param.kind {
            RestHirParamKind::Path => ParamSource::Path,
            RestHirParamKind::Query => ParamSource::Query,
            RestHirParamKind::Header => ParamSource::Header,
            RestHirParamKind::Cookie => ParamSource::Cookie,
            RestHirParamKind::Body => ParamSource::Body,
        },
        flatten: param.flatten,
    }
}
