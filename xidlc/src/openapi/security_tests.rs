use super::*;

macro_rules! test_fn {
    ($name:ident: $schema:expr; $expected:literal) => {
        #[test]
        fn $name() {
            let value = serde_json::to_value($schema).unwrap();
            let expected_value: serde_json::Value = serde_json::from_str($expected).unwrap();
            assert_eq!(value, expected_value);
        }
    };
}

test_fn! {
    security_scheme_correct_http_bearer_json:
    SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).bearer_format("JWT").build());
    r###"{ "type": "http", "scheme": "bearer", "bearerFormat": "JWT" }"###
}
test_fn! {
    security_scheme_correct_basic_auth:
    SecurityScheme::Http(Http::new(HttpAuthScheme::Basic));
    r###"{ "type": "http", "scheme": "basic" }"###
}
test_fn! {
    security_scheme_correct_digest_auth:
    SecurityScheme::Http(Http::new(HttpAuthScheme::Digest));
    r###"{ "type": "http", "scheme": "digest" }"###
}
test_fn! {
    security_scheme_correct_hoba_auth:
    SecurityScheme::Http(Http::new(HttpAuthScheme::Hoba));
    r###"{ "type": "http", "scheme": "hoba" }"###
}
test_fn! {
    security_scheme_correct_mutual_auth:
    SecurityScheme::Http(Http::new(HttpAuthScheme::Mutual));
    r###"{ "type": "http", "scheme": "mutual" }"###
}
test_fn! {
    security_scheme_correct_negotiate_auth:
    SecurityScheme::Http(Http::new(HttpAuthScheme::Negotiate));
    r###"{ "type": "http", "scheme": "negotiate" }"###
}
test_fn! {
    security_scheme_correct_oauth_auth:
    SecurityScheme::Http(Http::new(HttpAuthScheme::OAuth));
    r###"{ "type": "http", "scheme": "oauth" }"###
}
test_fn! {
    security_scheme_correct_scram_sha1_auth:
    SecurityScheme::Http(Http::new(HttpAuthScheme::ScramSha1));
    r###"{ "type": "http", "scheme": "scram-sha-1" }"###
}
test_fn! {
    security_scheme_correct_scram_sha256_auth:
    SecurityScheme::Http(Http::new(HttpAuthScheme::ScramSha256));
    r###"{ "type": "http", "scheme": "scram-sha-256" }"###
}
test_fn! {
    security_scheme_correct_api_key_cookie_auth:
    SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(String::from("api_key"))));
    r###"{ "type": "apiKey", "name": "api_key", "in": "cookie" }"###
}
test_fn! {
    security_scheme_correct_api_key_header_auth:
    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("api_key")));
    r###"{ "type": "apiKey", "name": "api_key", "in": "header" }"###
}
test_fn! {
    security_scheme_correct_api_key_query_auth:
    SecurityScheme::ApiKey(ApiKey::Query(ApiKeyValue::new(String::from("api_key"))));
    r###"{ "type": "apiKey", "name": "api_key", "in": "query" }"###
}
test_fn! {
    security_scheme_correct_open_id_connect_auth:
    SecurityScheme::OpenIdConnect(OpenIdConnect::new("https://localhost/openid"));
    r###"{ "type": "openIdConnect", "openIdConnectUrl": "https://localhost/openid" }"###
}
test_fn! {
    security_scheme_correct_mutual_tls:
    SecurityScheme::MutualTls { description: Some(String::from("authorization is performed with client side certificate")), extensions: None };
    r###"{ "type": "mutualTLS", "description": "authorization is performed with client side certificate" }"###
}
