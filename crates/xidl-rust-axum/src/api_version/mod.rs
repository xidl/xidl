//! Wire version identity: extracting the declared API version selector and
//! resolving it against the version a contract serves.

use http::HeaderMap;

#[cfg(test)]
mod tests;

/// Where the declared version selector travels on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersionLocation {
    /// The version travels in a request header.
    Header,
    /// The version travels as a request query parameter.
    Query,
}

impl ApiVersionLocation {
    /// Extracts the raw selector value a request declared, if any.
    pub fn extract(self, headers: &HeaderMap, uri: &http::Uri, name: &str) -> Option<String> {
        match self {
            Self::Header => headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            Self::Query => query_value(uri.query(), name),
        }
    }
}

/// A request declared a version the contract does not serve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedApiVersion {
    /// The raw value the request declared, when it declared one.
    pub requested: Option<String>,
    /// The wire value this contract serves.
    pub supported: String,
}

impl std::fmt::Display for UnsupportedApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.requested {
            Some(requested) => write!(
                f,
                "unsupported API version \"{requested}\": this contract serves {}",
                self.supported
            ),
            None => write!(
                f,
                "no API version declared: this contract serves {}",
                self.supported
            ),
        }
    }
}

impl std::error::Error for UnsupportedApiVersion {}

impl From<UnsupportedApiVersion> for crate::Error {
    fn from(error: UnsupportedApiVersion) -> Self {
        Self::new(400, error.to_string())
    }
}

/// The default resolution policy: only the exact supported version passes.
///
/// Anything else — another value, or no selector at all — fails with a typed
/// `400` instead of silently falling back to the latest version.
pub fn resolve_strict(raw: Option<&str>, supported: &str) -> Result<(), UnsupportedApiVersion> {
    match raw {
        Some(raw) if raw == supported => Ok(()),
        other => Err(UnsupportedApiVersion {
            requested: other.map(str::to_owned),
            supported: supported.to_owned(),
        }),
    }
}

/// Reads one query parameter out of a raw query string.
fn query_value(query: Option<&str>, name: &str) -> Option<String> {
    for pair in query?.split('&') {
        let (key, value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        if percent_decode(key) == name {
            return Some(percent_decode(value));
        }
    }
    None
}

/// Decodes `%XX` escapes; version values are plain tokens, so `+` stays as is.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = (bytes[index + 1] as char).to_digit(16);
            let low = (bytes[index + 2] as char).to_digit(16);
            if let (Some(high), Some(low)) = (high, low) {
                out.push((high * 16 + low) as u8);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(out)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}
