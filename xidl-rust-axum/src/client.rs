pub struct Client {
    base_url: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn url(&self, path: &str) -> String {
        if self.base_url.ends_with('/') && path.starts_with('/') {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        } else if !self.base_url.ends_with('/') && !path.starts_with('/') {
            format!("{}/{}", self.base_url, path)
        } else {
            format!("{}{}", self.base_url, path)
        }
    }
}
