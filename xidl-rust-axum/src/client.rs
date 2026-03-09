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
        self.join_url(path)
            .map(|url| url.to_string())
            .unwrap_or_else(|| self.concat_url(path))
    }

    pub fn ws_url(&self, path: &str) -> crate::Result<String> {
        let mut url = self
            .join_url(path)
            .ok_or_else(|| crate::Error::new(500, "invalid base_url"))?;
        match url.scheme() {
            "http" => {
                let _ = url.set_scheme("ws");
            }
            "https" => {
                let _ = url.set_scheme("wss");
            }
            _ => {
                return Err(crate::Error::new(
                    500,
                    "unsupported base_url scheme for websocket",
                ));
            }
        }
        Ok(url.to_string())
    }

    fn join_url(&self, path: &str) -> Option<reqwest::Url> {
        let mut base = reqwest::Url::parse(&self.base_url).ok()?;
        if !base.path().ends_with('/') {
            let path = format!("{}/", base.path());
            base.set_path(&path);
        }
        base.join(path.trim_start_matches('/')).ok()
    }

    fn concat_url(&self, path: &str) -> String {
        if self.base_url.ends_with('/') && path.starts_with('/') {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        } else if !self.base_url.ends_with('/') && !path.starts_with('/') {
            format!("{}/{}", self.base_url, path)
        } else {
            format!("{}{}", self.base_url, path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn url_join_preserves_base_path_prefix() {
        let client = Client::new("https://example.com/api");
        assert_eq!(client.url("/users"), "https://example.com/api/users");
    }

    #[test]
    fn ws_url_uses_websocket_scheme() {
        let client = Client::new("https://example.com/api");
        assert_eq!(
            client.ws_url("/chat").unwrap(),
            "wss://example.com/api/chat"
        );
    }
}
