#[cfg(test)]
mod tests;

use convert_case::{Case, Casing};
use jiff::Timestamp;
use minijinja::Error;

pub fn to_case(value: String, style: String) -> String {
    match style.as_str() {
        "UPPER_SNAKE" => value.to_case(Case::UpperSnake),
        "snake_case" => value.to_case(Case::Snake),
        _ => value,
    }
}

pub fn format_timestamp_filter(value: i64) -> std::result::Result<String, Error> {
    let timestamp = Timestamp::from_second(value).unwrap_or_default();

    Ok(timestamp
        .to_zoned(jiff::tz::TimeZone::UTC)
        .strftime("%Y-%m-%d %H:%M")
        .to_string())
}

pub fn strip_template_padding(input: String) -> String {
    let mut out = String::with_capacity(input.len());
    for (idx, line) in input.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(stripped) = line.strip_prefix("    ") {
            out.push_str(stripped);
        } else {
            out.push_str(line);
        }
    }
    out
}
