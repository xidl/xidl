use reqwest::header::{COOKIE, HeaderMap, HeaderValue};
use xidl_rust_axum::auth::basic::BasicAuth;
use xidl_rust_axum::{ApiKeyAuth, Client, ClientApiKeyLocation, ClientAuth, ClientAuthRequirement};

fn sample_auth() -> ClientAuth {
    ClientAuth {
        basic: Some(BasicAuth {
            username: "alice".to_string(),
            password: None,
        }),
        bearer: Some("bearer-token".to_string()),
        api_keys: vec![
            ApiKeyAuth {
                location: ClientApiKeyLocation::Header,
                name: "x-api-key".to_string(),
                value: "header-secret".to_string(),
            },
            ApiKeyAuth {
                location: ClientApiKeyLocation::Cookie,
                name: "session".to_string(),
                value: "cookie-secret".to_string(),
            },
            ApiKeyAuth {
                location: ClientApiKeyLocation::Query,
                name: "api_key".to_string(),
                value: "query-secret".to_string(),
            },
        ],
    }
}

#[test]
fn constructors_and_accessors_preserve_state() {
    let http = reqwest::Client::new();
    let auth = sample_auth();

    let client = Client::with_http("https://example.com/base", http.clone());
    assert_eq!(client.base_url(), "https://example.com/base");
    assert!(client.auth().is_none());

    let client = Client::with_http_and_auth("https://example.com/base", http, auth.clone());
    assert_eq!(client.base_url(), "https://example.com/base");
    assert!(client.auth().is_some());

    let overridden = client.with_auth_override(None);
    assert!(overridden.auth().is_none());
    let overridden = overridden.with_auth_override(Some(auth));
    assert!(overridden.auth().is_some());
    assert_eq!(overridden.base_url(), "https://example.com/base");
    let _ = overridden.http();
}

#[test]
fn api_key_lookup_returns_matching_entry() {
    let auth = sample_auth();
    let key = auth
        .api_key(ClientApiKeyLocation::Header, "x-api-key")
        .expect("missing api key");
    assert_eq!(key.value, "header-secret");
    assert!(
        auth.api_key(ClientApiKeyLocation::Header, "missing")
            .is_none()
    );
}

#[test]
fn apply_auth_headers_covers_basic_header_cookie_and_query_variants() {
    let client = Client::with_auth("https://example.com", sample_auth());
    let mut headers = HeaderMap::new();

    client
        .apply_auth_headers(&mut headers, ClientAuthRequirement::Basic)
        .unwrap();
    assert_eq!(
        headers.get(reqwest::header::AUTHORIZATION).unwrap(),
        "Basic YWxpY2U="
    );

    client
        .apply_auth_headers(
            &mut headers,
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Header,
                name: "x-api-key",
            },
        )
        .unwrap();
    assert_eq!(headers.get("x-api-key").unwrap(), "header-secret");

    headers.insert(COOKIE, HeaderValue::from_static("existing=1"));
    client
        .apply_auth_headers(
            &mut headers,
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Cookie,
                name: "session",
            },
        )
        .unwrap();
    assert_eq!(
        headers.get(COOKIE).unwrap(),
        "existing=1; session=cookie-secret"
    );

    headers.insert(COOKIE, HeaderValue::from_static(""));
    client
        .apply_auth_headers(
            &mut headers,
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Cookie,
                name: "session",
            },
        )
        .unwrap();
    assert_eq!(headers.get(COOKIE).unwrap(), "session=cookie-secret");

    client
        .apply_auth_headers(
            &mut headers,
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Query,
                name: "api_key",
            },
        )
        .unwrap();
}

#[test]
fn apply_auth_reports_missing_and_invalid_credentials() {
    let client = Client::new("https://example.com");
    let mut req = client
        .http()
        .get("https://example.com/data")
        .build()
        .unwrap();
    let err = client
        .apply_auth(&mut req, ClientAuthRequirement::Bearer)
        .unwrap_err();
    assert_eq!(err.code, 401);

    let invalid_header_name = Client::with_auth(
        "https://example.com",
        ClientAuth {
            basic: None,
            bearer: None,
            api_keys: vec![ApiKeyAuth {
                location: ClientApiKeyLocation::Header,
                name: "bad header".to_string(),
                value: "value".to_string(),
            }],
        },
    );
    let err = invalid_header_name
        .apply_auth_headers(
            &mut HeaderMap::new(),
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Header,
                name: "bad header",
            },
        )
        .unwrap_err();
    assert_eq!(err.code, 400);

    let invalid_cookie = Client::with_auth(
        "https://example.com",
        ClientAuth {
            basic: None,
            bearer: None,
            api_keys: vec![ApiKeyAuth {
                location: ClientApiKeyLocation::Cookie,
                name: "session".to_string(),
                value: "bad\ncookie".to_string(),
            }],
        },
    );
    let err = invalid_cookie
        .apply_auth_headers(
            &mut HeaderMap::new(),
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Cookie,
                name: "session",
            },
        )
        .unwrap_err();
    assert_eq!(err.code, 400);

    let mut req = invalid_header_name
        .http()
        .get("https://example.com/data")
        .build()
        .unwrap();
    let err = invalid_header_name
        .apply_auth(
            &mut req,
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Header,
                name: "bad header",
            },
        )
        .unwrap_err();
    assert_eq!(err.code, 400);
}

#[test]
fn apply_auth_to_ws_url_updates_query_and_surfaces_errors() {
    let client = Client::with_auth("https://example.com", sample_auth());
    let mut ws_url = "ws://example.com/chat".to_string();
    client
        .apply_auth_to_ws_url(
            &mut ws_url,
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Query,
                name: "api_key",
            },
        )
        .unwrap();
    assert!(ws_url.contains("api_key=query-secret"));

    let mut ws_url = "not a url".to_string();
    let err = client
        .apply_auth_to_ws_url(
            &mut ws_url,
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Query,
                name: "api_key",
            },
        )
        .unwrap_err();
    assert_eq!(err.code, 500);

    let client = Client::new("https://example.com");
    let mut ws_url = "ws://example.com/chat".to_string();
    let err = client
        .apply_auth_to_ws_url(
            &mut ws_url,
            ClientAuthRequirement::ApiKey {
                location: ClientApiKeyLocation::Query,
                name: "missing",
            },
        )
        .unwrap_err();
    assert_eq!(err.code, 401);

    let mut ws_url = "ws://example.com/chat".to_string();
    client
        .apply_auth_to_ws_url(&mut ws_url, ClientAuthRequirement::Bearer)
        .unwrap();
    assert_eq!(ws_url, "ws://example.com/chat");
}

#[test]
fn url_and_ws_url_cover_join_concat_and_scheme_validation() {
    let client = Client::new("https://example.com/base/");
    assert_eq!(client.url("/users"), "https://example.com/base/users");

    let client = Client::new("http://example.com/base");
    assert_eq!(
        client.ws_url("/chat").unwrap(),
        "ws://example.com/base/chat"
    );

    let invalid = Client::new("not a url");
    assert_eq!(invalid.url("/users"), "not a url/users");
    assert_eq!(invalid.url("users"), "not a url/users");

    let invalid = Client::new("not a url/");
    assert_eq!(invalid.url("/users"), "not a url/users");

    let ftp = Client::new("ftp://example.com/base");
    let err = ftp.ws_url("/chat").unwrap_err();
    assert_eq!(err.code, 500);
    assert_eq!(err.message, "unsupported base_url scheme for websocket");

    let invalid = Client::new("not a url");
    let err = invalid.ws_url("/chat").unwrap_err();
    assert_eq!(err.code, 500);
    assert_eq!(err.message, "invalid base_url");
}
