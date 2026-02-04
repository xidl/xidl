use chrono::Utc;
use convert_case::{Case, Casing};
use minijinja::{Error, ErrorKind, Value};

pub fn to_case(value: String, style: String) -> String {
    match style.as_str() {
        "UPPER_SNAKE" => value.to_case(Case::UpperSnake),
        _ => value,
    }
}

pub fn format_timestamp_filter(value: Value) -> std::result::Result<String, Error> {
    let secs: i64 = value.try_into().map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("timestamp must be an integer: {err}"),
        )
    })?;
    let dt = chrono::DateTime::<Utc>::from_timestamp(secs, 0).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("timestamp out of range: {secs}"),
        )
    })?;
    Ok(dt.format("%Y-%m-%d %H:%M").to_string())
}
