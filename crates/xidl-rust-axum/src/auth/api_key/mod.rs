use axum::http::HeaderMap;
use axum::http::Uri;

/// Location of an API key credential in an HTTP request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApiKeyLocation {
    /// Read the API key from a request header.
    Header,
    /// Read the API key from the query string.
    Query,
    /// Read the API key from the `Cookie` header.
    Cookie,
}

/// Extracted API key credential.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApiKeyAuth {
    /// Where the API key was found.
    pub location: ApiKeyLocation,
    /// Header, query parameter, or cookie name.
    pub name: String,
    /// Credential value.
    pub value: String,
}

/// Errors returned while extracting an API key credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyAuthError {
    /// No credential was found in the expected location.
    Missing,
    /// The header existed but was not valid UTF-8.
    InvalidHeaderValue,
}

/// Extracts an API key credential from a request's headers and URI.
pub fn extract_api_key(
    headers: &HeaderMap,
    uri: &Uri,
    location: ApiKeyLocation,
    name: &str,
) -> Result<ApiKeyAuth, ApiKeyAuthError> {
    match location {
        ApiKeyLocation::Header => extract_header_key(headers, name),
        ApiKeyLocation::Query => extract_query_key(uri, name),
        ApiKeyLocation::Cookie => extract_cookie_key(headers, name),
    }
}

fn extract_header_key(headers: &HeaderMap, name: &str) -> Result<ApiKeyAuth, ApiKeyAuthError> {
    let header = headers.get(name).ok_or(ApiKeyAuthError::Missing)?;
    let value = header
        .to_str()
        .map_err(|_| ApiKeyAuthError::InvalidHeaderValue)?;
    Ok(ApiKeyAuth {
        location: ApiKeyLocation::Header,
        name: name.to_string(),
        value: value.to_string(),
    })
}

fn extract_query_key(uri: &Uri, name: &str) -> Result<ApiKeyAuth, ApiKeyAuthError> {
    let query = uri.query().ok_or(ApiKeyAuthError::Missing)?;
    let map = serde_urlencoded::from_str::<std::collections::HashMap<String, String>>(query)
        .map_err(|_| ApiKeyAuthError::Missing)?;
    let value = map.get(name).cloned().ok_or(ApiKeyAuthError::Missing)?;
    Ok(ApiKeyAuth {
        location: ApiKeyLocation::Query,
        name: name.to_string(),
        value,
    })
}

fn extract_cookie_key(headers: &HeaderMap, name: &str) -> Result<ApiKeyAuth, ApiKeyAuthError> {
    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .ok_or(ApiKeyAuthError::Missing)?;
    let cookie_str = cookie_header
        .to_str()
        .map_err(|_| ApiKeyAuthError::InvalidHeaderValue)?;
    for part in cookie_str.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut iter = part.splitn(2, '=');
        let cookie_name = iter.next().unwrap_or("").trim();
        if cookie_name == name {
            let value = iter.next().unwrap_or("").trim().to_string();
            return Ok(ApiKeyAuth {
                location: ApiKeyLocation::Cookie,
                name: name.to_string(),
                value,
            });
        }
    }
    Err(ApiKeyAuthError::Missing)
}

#[cfg(test)]
mod tests;
