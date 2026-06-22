/// Decoded request payload together with the original HTTP headers.
///
/// Generated handlers use this wrapper when business logic needs access to
/// request metadata in addition to the deserialized body or parameters.
#[derive(Debug, Clone)]
pub struct Request<T> {
    /// Original request headers.
    pub headers: axum::http::HeaderMap,
    /// Decoded request payload.
    pub data: T,
}

impl<T> Request<T> {
    /// Creates a new request wrapper.
    pub fn new(headers: axum::http::HeaderMap, data: T) -> Self {
        Self { headers, data }
    }

    /// Returns the request headers.
    pub fn headers(&self) -> &axum::http::HeaderMap {
        &self.headers
    }

    /// Consumes the wrapper and returns the decoded payload.
    pub fn into_inner(self) -> T {
        self.data
    }
}

#[cfg(test)]
mod tests;
