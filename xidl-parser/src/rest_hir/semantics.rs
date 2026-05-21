mod annotation_parse;
mod annotations;
mod cors;
mod security;
mod stream;

#[cfg(test)]
mod tests;

use crate::hir;
use jiff::{Timestamp, civil, tz::TimeZone};
use serde::{Deserialize, Serialize};

pub use self::annotations::{
    annotation_name, annotation_params, effective_media_type, find_annotation, has_annotation,
    has_optional_annotation, normalize_annotation_params,
};
pub use self::cors::{HttpCorsProfile, effective_cors};
pub use self::security::{
    HttpApiKeyLocation, HttpSecurityOrigin, HttpSecurityProfile, HttpSecurityRequirement,
    effective_security, effective_security_with_origin,
};
pub use self::stream::{
    HttpStreamCodec, HttpStreamConfig, HttpStreamKind, HttpStreamTargetSupport, http_stream_config,
    validate_http_stream_method, validate_http_stream_target,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeprecatedInfo {
    pub deprecated: bool,
    pub since: Option<String>,
    pub after: Option<String>,
}

pub fn deprecated_info(annotations: &[hir::Annotation]) -> Result<Option<DeprecatedInfo>, String> {
    let annotation = annotations.iter().find(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("deprecated"))
            .unwrap_or(false)
    });
    let Some(annotation) = annotation else {
        return Ok(None);
    };
    let mut since = None;
    let mut after = None;
    if let Some(params) = annotation_params(annotation) {
        let params = normalize_annotation_params(params);
        if let Some(value) = params.get("value") {
            since = Some(normalize_deprecated_timestamp(value, false)?);
        }
        if let Some(value) = params.get("since") {
            since = Some(normalize_deprecated_timestamp(value, false)?);
        }
        if let Some(value) = params.get("after") {
            after = Some(normalize_deprecated_timestamp(value, true)?);
        }
    }
    if let (Some(since), Some(after)) = (&since, &after) {
        validate_deprecated_range(since, after)?;
    }
    Ok(Some(DeprecatedInfo {
        deprecated: true,
        since,
        after,
    }))
}

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
        let canonical = if annotations::media_type_annotation_aliases("Consumes")
            .iter()
            .any(|alias| name.eq_ignore_ascii_case(alias))
        {
            "Consumes"
        } else if annotations::media_type_annotation_aliases("Produces")
            .iter()
            .any(|alias| name.eq_ignore_ascii_case(alias))
        {
            "Produces"
        } else {
            continue;
        };
        let Some(value) =
            annotations::annotation_value(std::slice::from_ref(annotation), canonical)
        else {
            continue;
        };
        if is_supported_http_media_type(&value) {
            continue;
        }
        return Err(format!(
            "{target}: unsupported @{name}(\"{value}\") media type"
        ));
    }
    Ok(())
}

fn is_supported_http_media_type(value: &str) -> bool {
    value.eq_ignore_ascii_case("application/json")
        || value.eq_ignore_ascii_case("application/x-www-form-urlencoded")
        || value.eq_ignore_ascii_case("application/msgpack")
        || value.eq_ignore_ascii_case("text/plain")
}

fn validate_deprecated_range(since: &str, after: &str) -> Result<(), String> {
    let since_ts: Timestamp = since
        .parse()
        .map_err(|_| format!("invalid @deprecated(since) timestamp '{since}'"))?;
    let after_ts: Timestamp = after
        .parse()
        .map_err(|_| format!("invalid @deprecated(after) timestamp '{after}'"))?;
    if since_ts > after_ts {
        return Err("@deprecated(since=..., after=...) requires since <= after".to_string());
    }
    Ok(())
}

fn normalize_deprecated_timestamp(value: &str, end_of_day: bool) -> Result<String, String> {
    if let Ok(ts) = value.parse::<Timestamp>() {
        return Ok(ts.to_zoned(TimeZone::UTC).timestamp().to_string());
    }
    let date: civil::Date = value
        .parse()
        .map_err(|_| format!("invalid @deprecated timestamp literal '{value}'"))?;
    let dt = if end_of_day {
        date.at(23, 59, 59, 0)
    } else {
        date.to_datetime(civil::Time::midnight())
    };
    let zoned = dt.to_zoned(TimeZone::UTC).map_err(|err| err.to_string())?;
    Ok(zoned.timestamp().to_string())
}
