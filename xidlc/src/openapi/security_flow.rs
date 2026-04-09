use crate::openapi::extensions::Extensions;
use serde::{Deserialize, Serialize};

use super::Scopes;

#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Implicit {
    pub authorization_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    #[serde(flatten)]
    pub scopes: Scopes,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub extensions: Option<Extensions>,
}

impl Implicit {
    pub fn new<S: Into<String>>(authorization_url: S, scopes: Scopes) -> Self {
        Self {
            authorization_url: authorization_url.into(),
            refresh_url: None,
            scopes,
            extensions: Default::default(),
        }
    }

    pub fn with_refresh_url<S: Into<String>>(
        authorization_url: S,
        scopes: Scopes,
        refresh_url: S,
    ) -> Self {
        Self {
            authorization_url: authorization_url.into(),
            refresh_url: Some(refresh_url.into()),
            scopes,
            extensions: Default::default(),
        }
    }
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationCode {
    pub authorization_url: String,
    pub token_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    #[serde(flatten)]
    pub scopes: Scopes,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub extensions: Option<Extensions>,
}

impl AuthorizationCode {
    pub fn new<A: Into<String>, T: Into<String>>(
        authorization_url: A,
        token_url: T,
        scopes: Scopes,
    ) -> Self {
        Self {
            authorization_url: authorization_url.into(),
            token_url: token_url.into(),
            refresh_url: None,
            scopes,
            extensions: Default::default(),
        }
    }

    pub fn with_refresh_url<S: Into<String>>(
        authorization_url: S,
        token_url: S,
        scopes: Scopes,
        refresh_url: S,
    ) -> Self {
        Self {
            authorization_url: authorization_url.into(),
            token_url: token_url.into(),
            refresh_url: Some(refresh_url.into()),
            scopes,
            extensions: Default::default(),
        }
    }
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Password {
    pub token_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    #[serde(flatten)]
    pub scopes: Scopes,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub extensions: Option<Extensions>,
}

impl Password {
    pub fn new<S: Into<String>>(token_url: S, scopes: Scopes) -> Self {
        Self {
            token_url: token_url.into(),
            refresh_url: None,
            scopes,
            extensions: Default::default(),
        }
    }

    pub fn with_refresh_url<S: Into<String>>(token_url: S, scopes: Scopes, refresh_url: S) -> Self {
        Self {
            token_url: token_url.into(),
            refresh_url: Some(refresh_url.into()),
            scopes,
            extensions: Default::default(),
        }
    }
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientCredentials {
    pub token_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    #[serde(flatten)]
    pub scopes: Scopes,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub extensions: Option<Extensions>,
}

impl ClientCredentials {
    pub fn new<S: Into<String>>(token_url: S, scopes: Scopes) -> Self {
        Self {
            token_url: token_url.into(),
            refresh_url: None,
            scopes,
            extensions: Default::default(),
        }
    }

    pub fn with_refresh_url<S: Into<String>>(token_url: S, scopes: Scopes, refresh_url: S) -> Self {
        Self {
            token_url: token_url.into(),
            refresh_url: Some(refresh_url.into()),
            scopes,
            extensions: Default::default(),
        }
    }
}
