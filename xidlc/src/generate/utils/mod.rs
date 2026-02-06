use convert_case::{Case, Casing};
use jiff::{Timestamp, Zoned};
use minijinja::{Error, Value};
use std::str::FromStr;

pub fn to_case(value: String, style: String) -> String {
    match style.as_str() {
        "UPPER_SNAKE" => value.to_case(Case::UpperSnake),
        _ => value,
    }
}

pub fn format_timestamp_filter(value: Value) -> std::result::Result<String, Error> {
    Ok(parse_timestamp(value)
        .unwrap_or_else(epoch_timestamp)
        .strftime("%Y-%m-%d %H:%M")
        .to_string())
}

fn parse_timestamp(value: Value) -> Option<Zoned> {
    let maybe_secs: Result<i64, _> = value.clone().try_into();
    if let Ok(secs) = maybe_secs {
        return from_numeric_timestamp(secs);
    }

    let raw: String = value.try_into().ok()?;
    let trimmed = raw.trim();

    if let Ok(number) = trimmed.parse::<i64>()
        && let Some(zoned) = from_numeric_timestamp(number)
    {
        return Some(zoned);
    }

    Timestamp::from_str(trimmed)
        .ok()
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC))
}

fn from_numeric_timestamp(value: i64) -> Option<Zoned> {
    let timestamp = if value.abs() >= 1_000_000_000_000 {
        Timestamp::from_millisecond(value)
    } else {
        Timestamp::from_second(value)
    };
    timestamp
        .ok()
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC))
}

fn epoch_timestamp() -> Zoned {
    Timestamp::from_second(0)
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC))
        .expect("unix epoch must be valid")
}
