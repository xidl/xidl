use jiff::Timestamp;
use minijinja::{Error, ErrorKind};

pub fn format_timestamp_filter(value: i64) -> std::result::Result<String, Error> {
    let timestamp = Timestamp::from_second(value).unwrap_or_default();

    Ok(timestamp
        .to_zoned(jiff::tz::TimeZone::UTC)
        .strftime("%Y-%m-%d %H:%M")
        .to_string())
}

pub fn rust_format_filter(value: String) -> std::result::Result<String, Error> {
    crate::fmt::format_rust_source(&value).map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("rust format failed: {err}"),
        )
    })
}

#[cfg(feature = "gen-typescript")]
pub fn typescript_format_filter(value: String) -> std::result::Result<String, Error> {
    crate::fmt::format_typescript_source(&value).map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("typescript format failed: {err}"),
        )
    })
}
