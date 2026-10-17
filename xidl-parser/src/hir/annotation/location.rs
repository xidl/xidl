use serde::{Deserialize, Serialize};

/// Locations supported by named API key and API version parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpParameterLocation {
    /// An HTTP request header; names ignore ASCII case.
    Header,
    /// A URL query parameter; names are case-sensitive.
    Query,
    /// An HTTP cookie; names are case-sensitive.
    Cookie,
}

impl HttpParameterLocation {
    /// Recognizes the annotation spelling, ignoring ASCII case.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "header" => Some(Self::Header),
            "query" => Some(Self::Query),
            "cookie" => Some(Self::Cookie),
            _ => None,
        }
    }

    /// Returns the spelling used by HTTP annotations and OpenAPI parameters.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Query => "query",
            Self::Cookie => "cookie",
        }
    }

    /// Compares wire names using this location's protocol rule.
    pub fn names_equal(self, first: &str, second: &str) -> bool {
        match self {
            Self::Header => first.eq_ignore_ascii_case(second),
            Self::Query | Self::Cookie => first == second,
        }
    }

    /// Checks a name before it is emitted in a URL, header or cookie.
    pub fn valid_name(self, name: &str) -> bool {
        match self {
            Self::Header | Self::Cookie => http::HeaderName::from_bytes(name.as_bytes()).is_ok(),
            Self::Query => !name.is_empty() && !name.chars().any(char::is_control),
        }
    }
}

/// The location and wire name declared by `@version(in = ..., name = ...)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionParameter {
    /// Where generated clients carry their module's version value.
    pub location: HttpParameterLocation,
    /// The parameter's name at that location.
    pub name: String,
}

impl VersionParameter {
    pub(crate) fn validate_value(&self, value: &str) -> Result<(), String> {
        if self.location == HttpParameterLocation::Header {
            if value.trim_matches([' ', '\t']) != value {
                return Err(
                    "version header value cannot start or end with HTTP whitespace".to_string(),
                );
            }
            let header = http::HeaderValue::from_str(value)
                .map_err(|error| format!("invalid version header value: {error}"))?;
            header
                .to_str()
                .map_err(|error| format!("version header value must be visible ASCII: {error}"))?;
        }
        Ok(())
    }
}
