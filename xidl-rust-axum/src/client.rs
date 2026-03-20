use crate::Error;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::header::{AUTHORIZATION, COOKIE, HeaderMap, HeaderValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientApiKeyLocation {
    Header,
    Query,
    Cookie,
}

#[derive(Clone, Debug)]
pub struct ApiKeyAuth {
    pub location: ClientApiKeyLocation,
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default)]
pub struct ClientAuth {
    pub basic: Option<crate::auth::basic::BasicAuth>,
    pub bearer: Option<String>,
    pub api_keys: Vec<ApiKeyAuth>,
}

impl ClientAuth {
    pub fn api_key(&self, location: ClientApiKeyLocation, name: &str) -> Option<&ApiKeyAuth> {
        self.api_keys
            .iter()
            .find(|key| key.location == location && key.name == name)
    }
}

#[derive(Clone, Debug)]
pub enum ClientAuthRequirement<'a> {
    Basic,
    Bearer,
    ApiKey {
        location: ClientApiKeyLocation,
        name: &'a str,
    },
}

pub struct Client {
    base_url: String,
    http: reqwest::Client,
    auth: Option<ClientAuth>,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
            auth: None,
        }
    }

    pub fn with_auth(base_url: impl Into<String>, auth: ClientAuth) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
            auth: Some(auth),
        }
    }

    pub fn with_http(base_url: impl Into<String>, http: reqwest::Client) -> Self {
        Self {
            base_url: base_url.into(),
            http,
            auth: None,
        }
    }

    pub fn with_http_and_auth(
        base_url: impl Into<String>,
        http: reqwest::Client,
        auth: ClientAuth,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            http,
            auth: Some(auth),
        }
    }

    pub fn with_auth_override(&self, auth: Option<ClientAuth>) -> Self {
        Self {
            base_url: self.base_url.clone(),
            http: self.http.clone(),
            auth,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn auth(&self) -> Option<&ClientAuth> {
        self.auth.as_ref()
    }

    pub fn apply_auth(
        &self,
        req: &mut reqwest::Request,
        requirement: ClientAuthRequirement<'_>,
    ) -> crate::Result<()> {
        match requirement {
            ClientAuthRequirement::ApiKey { location, name } => {
                if location == ClientApiKeyLocation::Query {
                    self.apply_api_key_query(req, name)
                } else {
                    self.apply_auth_headers(req.headers_mut(), requirement)
                }
            }
            _ => self.apply_auth_headers(req.headers_mut(), requirement),
        }
    }

    fn apply_api_key_query(&self, req: &mut reqwest::Request, name: &str) -> crate::Result<()> {
        let auth = self
            .auth
            .as_ref()
            .and_then(|auth| auth.api_key(ClientApiKeyLocation::Query, name))
            .ok_or_else(|| Error::unauthorized())?;
        req.url_mut()
            .query_pairs_mut()
            .append_pair(&auth.name, &auth.value);
        Ok(())
    }

    pub fn apply_auth_headers(
        &self,
        headers: &mut HeaderMap,
        requirement: ClientAuthRequirement<'_>,
    ) -> crate::Result<()> {
        match requirement {
            ClientAuthRequirement::Basic => {
                let auth = self
                    .auth
                    .as_ref()
                    .and_then(|auth| auth.basic.as_ref())
                    .ok_or_else(|| Error::unauthorized())?;
                let header_value = basic_auth_header_value(auth)?;
                headers.insert(AUTHORIZATION, header_value);
                Ok(())
            }
            ClientAuthRequirement::Bearer => {
                let token = self
                    .auth
                    .as_ref()
                    .and_then(|auth| auth.bearer.as_deref())
                    .ok_or_else(|| Error::unauthorized())?;
                let value = format!("Bearer {token}");
                let header_value = HeaderValue::from_str(&value)
                    .map_err(|err| Error::new(400, format!("{err:?}")))?;
                headers.insert(AUTHORIZATION, header_value);
                Ok(())
            }
            ClientAuthRequirement::ApiKey { location, name } => {
                let auth = self
                    .auth
                    .as_ref()
                    .and_then(|auth| auth.api_key(location, name))
                    .ok_or_else(|| Error::unauthorized())?;
                match auth.location {
                    ClientApiKeyLocation::Header => {
                        let header_name =
                            reqwest::header::HeaderName::from_bytes(auth.name.as_bytes())
                                .map_err(|err| Error::new(400, format!("{err:?}")))?;
                        let header_value = HeaderValue::from_str(&auth.value)
                            .map_err(|err| Error::new(400, format!("{err:?}")))?;
                        headers.insert(header_name, header_value);
                        Ok(())
                    }
                    ClientApiKeyLocation::Cookie => {
                        let cookie_pair = format!("{}={}", auth.name, auth.value);
                        let merged = match headers.get(COOKIE) {
                            Some(existing) => {
                                let existing = existing
                                    .to_str()
                                    .map_err(|err| Error::new(400, format!("{err:?}")))?;
                                if existing.is_empty() {
                                    cookie_pair
                                } else {
                                    format!("{existing}; {cookie_pair}")
                                }
                            }
                            None => cookie_pair,
                        };
                        let header_value = HeaderValue::from_str(&merged)
                            .map_err(|err| Error::new(400, format!("{err:?}")))?;
                        headers.insert(COOKIE, header_value);
                        Ok(())
                    }
                    ClientApiKeyLocation::Query => Ok(()),
                }
            }
        }
    }

    pub fn apply_auth_to_ws_url(
        &self,
        ws_url: &mut String,
        requirement: ClientAuthRequirement<'_>,
    ) -> crate::Result<()> {
        if let ClientAuthRequirement::ApiKey { location, name } = requirement {
            if location == ClientApiKeyLocation::Query {
                let auth = self
                    .auth
                    .as_ref()
                    .and_then(|auth| auth.api_key(location, name))
                    .ok_or_else(|| Error::unauthorized())?;
                let mut url = reqwest::Url::parse(ws_url)
                    .map_err(|err| Error::new(500, format!("{err:?}")))?;
                url.query_pairs_mut().append_pair(&auth.name, &auth.value);
                *ws_url = url.to_string();
            }
        }
        Ok(())
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

fn basic_auth_header_value(auth: &crate::auth::basic::BasicAuth) -> crate::Result<HeaderValue> {
    let token = match &auth.password {
        Some(pass) => format!("{}:{}", auth.username, pass),
        None => auth.username.clone(),
    };
    let encoded = STANDARD.encode(token.as_bytes());
    let value = format!("Basic {encoded}");
    HeaderValue::from_str(&value).map_err(|err| Error::new(400, format!("{err:?}")))
}

#[cfg(test)]
mod tests {
    use super::Client;
    use super::{ApiKeyAuth, ClientApiKeyLocation, ClientAuth, ClientAuthRequirement};
    use crate::auth::basic::BasicAuth;

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

    #[test]
    fn apply_bearer_auth_sets_authorization_header() {
        let auth = ClientAuth {
            basic: None,
            bearer: Some("token-123".to_string()),
            api_keys: Vec::new(),
        };
        let client = Client::with_auth("https://example.com", auth);
        let mut req = client
            .http()
            .get("https://example.com/data")
            .build()
            .unwrap();
        client
            .apply_auth(&mut req, ClientAuthRequirement::Bearer)
            .unwrap();
        let value = req
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(value, "Bearer token-123");
    }

    #[test]
    fn apply_basic_auth_sets_authorization_header() {
        let auth = ClientAuth {
            basic: Some(BasicAuth {
                username: "user".to_string(),
                password: Some("pass".to_string()),
            }),
            bearer: None,
            api_keys: Vec::new(),
        };
        let client = Client::with_auth("https://example.com", auth);
        let mut req = client
            .http()
            .get("https://example.com/data")
            .build()
            .unwrap();
        client
            .apply_auth(&mut req, ClientAuthRequirement::Basic)
            .unwrap();
        let value = req
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(value.starts_with("Basic "));
    }

    #[test]
    fn apply_api_key_query_sets_query_param() {
        let auth = ClientAuth {
            basic: None,
            bearer: None,
            api_keys: vec![ApiKeyAuth {
                location: ClientApiKeyLocation::Query,
                name: "api_key".to_string(),
                value: "secret".to_string(),
            }],
        };
        let client = Client::with_auth("https://example.com", auth);
        let mut req = client
            .http()
            .get("https://example.com/data")
            .build()
            .unwrap();
        client
            .apply_auth(
                &mut req,
                ClientAuthRequirement::ApiKey {
                    location: ClientApiKeyLocation::Query,
                    name: "api_key",
                },
            )
            .unwrap();
        let query = req.url().query().unwrap_or_default();
        assert!(query.contains("api_key=secret"));
    }
}
