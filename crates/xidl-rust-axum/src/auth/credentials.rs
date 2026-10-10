use super::api_key::{ApiKeyAuth, ApiKeyLocation};

/// Authentication material shared by generated clients and server handlers.
#[derive(Clone, Debug, Default)]
pub struct ClientAuth {
    /// Basic auth credential.
    pub basic: Option<crate::auth::basic::BasicAuth>,
    /// Bearer token without the `Bearer ` prefix.
    pub bearer: Option<String>,
    /// Available API keys.
    pub api_keys: Vec<ApiKeyAuth>,
}

impl ClientAuth {
    /// Finds an API key matching the required location and name.
    pub fn api_key(&self, location: ApiKeyLocation, name: &str) -> Option<&ApiKeyAuth> {
        self.api_keys
            .iter()
            .find(|key| key.location == location && key.name == name)
    }
}
