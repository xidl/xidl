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
    security_scheme_correct_oauth2_implicit:
    SecurityScheme::OAuth2(OAuth2::with_description([Flow::Implicit(Implicit::new("https://localhost/auth/dialog", Scopes::from_iter([("edit:items", "edit my items"), ("read:items", "read my items")])))] , "my oauth2 flow"));
    r###"{ "type": "oauth2", "flows": { "implicit": { "authorizationUrl": "https://localhost/auth/dialog", "scopes": { "edit:items": "edit my items", "read:items": "read my items" } } }, "description": "my oauth2 flow" }"###
}
test_fn! {
    security_scheme_correct_oauth2_password:
    SecurityScheme::OAuth2(OAuth2::with_description([Flow::Password(Password::with_refresh_url("https://localhost/oauth/token", Scopes::from_iter([("edit:items", "edit my items"), ("read:items", "read my items")]), "https://localhost/refresh/token"))], "my oauth2 flow"));
    r###"{ "type": "oauth2", "flows": { "password": { "tokenUrl": "https://localhost/oauth/token", "refreshUrl": "https://localhost/refresh/token", "scopes": { "edit:items": "edit my items", "read:items": "read my items" } } }, "description": "my oauth2 flow" }"###
}
test_fn! {
    security_scheme_correct_oauth2_client_credentials:
    SecurityScheme::OAuth2(OAuth2::new([Flow::ClientCredentials(ClientCredentials::with_refresh_url("https://localhost/oauth/token", Scopes::from_iter([("edit:items", "edit my items"), ("read:items", "read my items")]), "https://localhost/refresh/token"))]));
    r###"{ "type": "oauth2", "flows": { "clientCredentials": { "tokenUrl": "https://localhost/oauth/token", "refreshUrl": "https://localhost/refresh/token", "scopes": { "edit:items": "edit my items", "read:items": "read my items" } } } }"###
}
test_fn! {
    security_scheme_correct_oauth2_authorization_code:
    SecurityScheme::OAuth2(OAuth2::new([Flow::AuthorizationCode(AuthorizationCode::with_refresh_url("https://localhost/authorization/token", "https://localhost/token/url", Scopes::from_iter([("edit:items", "edit my items"), ("read:items", "read my items")]), "https://localhost/refresh/token"))]));
    r###"{ "type": "oauth2", "flows": { "authorizationCode": { "authorizationUrl": "https://localhost/authorization/token", "tokenUrl": "https://localhost/token/url", "refreshUrl": "https://localhost/refresh/token", "scopes": { "edit:items": "edit my items", "read:items": "read my items" } } } }"###
}
test_fn! {
    security_scheme_correct_oauth2_authorization_code_no_scopes:
    SecurityScheme::OAuth2(OAuth2::new([Flow::AuthorizationCode(AuthorizationCode::with_refresh_url("https://localhost/authorization/token", "https://localhost/token/url", Scopes::new(), "https://localhost/refresh/token"))]));
    r###"{ "type": "oauth2", "flows": { "authorizationCode": { "authorizationUrl": "https://localhost/authorization/token", "tokenUrl": "https://localhost/token/url", "refreshUrl": "https://localhost/refresh/token", "scopes": {} } } }"###
}
