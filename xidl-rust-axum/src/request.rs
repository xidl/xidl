#[derive(Debug, Clone)]
pub struct Request<T> {
    pub headers: axum::http::HeaderMap,
    pub data: T,
}

impl<T> Request<T> {
    pub fn new(headers: axum::http::HeaderMap, data: T) -> Self {
        Self { headers, data }
    }

    pub fn headers(&self) -> &axum::http::HeaderMap {
        &self.headers
    }

    pub fn into_inner(self) -> T {
        self.data
    }
}
