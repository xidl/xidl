use axum::http::{HeaderMap, header};

pub fn accepts_media_type(headers: &HeaderMap, expected: &str) -> bool {
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
            if media.eq_ignore_ascii_case(expected) {
                return true;
            }
            if let Some((ty, sub)) = media.split_once('/') {
                if sub == "*" && ty.eq_ignore_ascii_case(expected_type.0) {
                    return true;
                }
            }
        }
    }
    false
}

fn parse_media_type(value: &str) -> Option<(&str, &str)> {
    let media = value.split(';').next()?.trim();
    media.split_once('/')
}

pub mod serde_ext {
    use serde::de::{self, Deserializer};
    use serde::Deserialize;

    pub fn default_on_missing_reject_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: serde::Deserialize<'de>,
    {
        Option::<T>::deserialize(deserializer)?
            .ok_or_else(|| de::Error::custom("null is not allowed"))
    }
}
