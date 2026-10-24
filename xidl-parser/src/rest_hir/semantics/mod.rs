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
use jiff::{Timestamp, civil, tz::TimeZone};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeprecatedInfo {
    pub deprecated: bool,
    pub since: Option<String>,
    pub after: Option<String>,
}

impl DeprecatedInfo {
    /// The human-readable note every target renders for this deprecation.
    pub fn note(&self) -> String {
        match self.detail() {
            Some(detail) => format!("Deprecated. {detail}"),
            None => String::from("Deprecated."),
        }
    }

    /// The JSDoc tag every TypeScript surface renders for this deprecation.
    pub fn tag(&self) -> String {
        match self.detail() {
            Some(detail) => format!("@deprecated {detail}"),
            None => String::from("@deprecated"),
        }
    }

    /// The description line recording one deprecated enumerator, for targets
    /// whose enum values carry no per-value deprecation flag.
    pub fn enumerator_note(&self, wire_name: &str) -> String {
        let mut note = format!("`{wire_name}` is deprecated");
        if let Some(since) = &self.since {
            note.push_str(&format!(" since {since}"));
        }
        if let Some(after) = &self.after {
            note.push_str(&format!(" after {after}"));
        }
        note.push('.');
        note
    }

    /// The `since`/`after` sentences, when either was declared.
    fn detail(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(since) = &self.since {
            parts.push(format!("Since {since}."));
        }
        if let Some(after) = &self.after {
            parts.push(format!("After {after}."));
        }
        (!parts.is_empty()).then(|| parts.join(" "))
    }
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

/// The deprecation that applies to a declaration: its own annotation when
/// present, else the nearest enclosing one. Levels run from the declaration
/// outward (member, type, module), so the most specific annotation wins.
pub fn effective_deprecated(
    levels: &[&[hir::Annotation]],
) -> Result<Option<DeprecatedInfo>, String> {
    for annotations in levels {
        if let Some(info) = deprecated_info(annotations)? {
            return Ok(Some(info));
        }
    }
    Ok(None)
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
