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
