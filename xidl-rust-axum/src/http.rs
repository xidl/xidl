use axum::http::{HeaderMap, header};

/// Returns `true` when the request accepts `expected` according to `Accept`.
///
/// Missing `Accept` headers are treated as accepting any media type.
pub fn accepts_media_type(headers: &HeaderMap, expected: &str) -> bool {
    let Some(expected) = canonical_media_type(expected) else {
        return false;
    };
    let Some(expected_type) = parse_media_type(expected) else {
        return false;
    };
    let values = headers.get_all(header::ACCEPT);
    if values.iter().next().is_none() {
        return true;
    }
    for value in values {
        let Ok(value) = value.to_str() else {
            continue;
        };
        for item in value.split(',') {
            let media = item.split(';').next().unwrap_or("").trim();
            if media.is_empty() || media == "*/*" {
                return true;
            }
            if media_type_eq(media, expected) {
                return true;
            }
            if let Some((ty, sub)) = parse_media_type(media) {
                if sub == "*" && ty.eq_ignore_ascii_case(expected_type.0) {
                    return true;
                }
            }
        }
    }
    false
}

/// Returns `true` when `Content-Type` matches `expected`, ignoring parameters.
pub fn content_type_matches(headers: &HeaderMap, expected: &str) -> bool {
    let Some(content_type) = headers.get(header::CONTENT_TYPE) else {
        return false;
    };
    let Ok(content_type) = content_type.to_str() else {
        return false;
    };
    media_type_eq(content_type, expected)
}

/// Compares two media types case-insensitively after canonicalization.
pub fn media_type_eq(actual: &str, expected: &str) -> bool {
    match (canonical_media_type(actual), canonical_media_type(expected)) {
        (Some(actual), Some(expected)) => actual.eq_ignore_ascii_case(expected),
        _ => false,
    }
}

/// Extracts the bare media type without parameters.
pub fn canonical_media_type(value: &str) -> Option<&str> {
    let media = value.split(';').next()?.trim();
    if media.contains('/') {
        Some(media)
    } else {
        None
    }
}

fn parse_media_type(value: &str) -> Option<(&str, &str)> {
    let media = canonical_media_type(value)?;
    media.split_once('/')
}

/// Serde helper functions used by generated request models.
pub mod serde_ext {
    use serde::Deserialize;
    use serde::de::{self, Deserializer};

    /// Uses the field default when absent, but rejects explicit `null`.
    pub fn default_on_missing_reject_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: serde::Deserialize<'de>,
    {
        Option::<T>::deserialize(deserializer)?
            .ok_or_else(|| de::Error::custom("null is not allowed"))
    }
}

#[cfg(test)]
mod tests;
