mod annotation_parse;
mod annotations;
mod cors;
mod security;
mod source;
pub(crate) use source::SourceBinding;
mod stream;
mod upgrade;

#[cfg(test)]
mod tests;

use crate::hir;

pub use self::annotations::{
    annotation_name, annotation_params, effective_media_type, find_annotation, has_annotation,
    has_optional_annotation, normalize_annotation_params,
};
pub use self::cors::{HttpCorsProfile, effective_cors};
pub use self::security::{
    HttpSecurityOrigin, HttpSecurityProfile, HttpSecurityRequirement, effective_security,
    effective_security_with_origin,
};
pub use self::stream::{
    HttpStreamCodec, HttpStreamConfig, HttpStreamKind, HttpStreamTargetSupport, http_stream_config,
    validate_http_stream_method, validate_http_stream_target,
};
pub use self::upgrade::{
    UpgradeMode, WebSocketCodec, WebSocketConfig, classify_upgrade_protocol, parse_duration_ms,
    parse_upgrade_protocol, parse_websocket_config, validate_websocket_subprotocol,
};
pub use crate::hir::{DeprecatedInfo, deprecated_info, effective_deprecated};

pub fn validate_http_annotations(
    target: &str,
    annotations: &[hir::Annotation],
) -> Result<(), String> {
    let _ = cors::collect_cors(annotations).map_err(|err| format!("{target}: {err}"))?;
    let _ = deprecated_info(annotations).map_err(|err| format!("{target}: {err}"))?;
    let _ = security::collect_security(annotations).map_err(|err| format!("{target}: {err}"))?;
    validate_rest_media_types(target, annotations)?;
    Ok(())
}

fn validate_rest_media_types(target: &str, annotations: &[hir::Annotation]) -> Result<(), String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let canonical = if name.eq_ignore_ascii_case("request") {
            Some("request")
        } else if name.eq_ignore_ascii_case("response") {
            Some("response")
        } else {
            None
        };
        let Some(canonical) = canonical else {
            continue;
        };
        let Some(value) =
            annotations::annotation_value(std::slice::from_ref(annotation), canonical)
        else {
            continue;
        };
        if annotations::media_type_token_to_mime(&value).is_some() {
            continue;
        }
        return Err(format!(
            "{target}: unsupported @{name}(\"{value}\") format, expected 'json', 'urlencoded', or 'msgpack'"
        ));
    }
    Ok(())
}
