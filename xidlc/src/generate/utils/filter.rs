use minijinja::{Error, ErrorKind};

pub fn rust_format_filter(value: String) -> std::result::Result<String, Error> {
    crate::fmt::format_rust_source(&value).map_err(|err| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("rust format failed: {err}"),
        )
    })
}

pub fn is_upper_case(value: &str) -> std::result::Result<bool, Error> {
    for ch in value.chars() {
        if ch.is_alphabetic() && ch.is_lowercase() {
            return Ok(false);
        }
    }

    Ok(true)
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
