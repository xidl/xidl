use std::{collections::BTreeMap, iter};

use crate::openapi::extensions::Extensions;
use serde::{Deserialize, Serialize};

use super::security_http::{Http, OpenIdConnect};
use super::security_oauth::OAuth2;

#[non_exhaustive]
#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct SecurityRequirement {
    #[serde(flatten)]
    value: BTreeMap<String, Vec<String>>,
}

impl SecurityRequirement {
    pub fn new<N: Into<String>, S: IntoIterator<Item = I>, I: Into<String>>(
        name: N,
        scopes: S,
    ) -> Self {
        Self {
            value: BTreeMap::from_iter(iter::once_with(|| {
                (
                    Into::<String>::into(name),
                    scopes
                        .into_iter()
                        .map(Into::<String>::into)
                        .collect::<Vec<_>>(),
                )
            })),
        }
    }

    pub fn add<N: Into<String>, S: IntoIterator<Item = I>, I: Into<String>>(
        mut self,
        name: N,
        scopes: S,
    ) -> Self {
        self.value.insert(
            Into::<String>::into(name),
            scopes.into_iter().map(Into::<String>::into).collect(),
        );
        self
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecurityScheme {
    #[serde(rename = "oauth2")]
    OAuth2(OAuth2),
    ApiKey(ApiKey),
    Http(Http),
    OpenIdConnect(OpenIdConnect),
    #[serde(rename = "mutualTLS")]
    MutualTls {
        #[allow(missing_docs)]
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        extensions: Option<Extensions>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "in", rename_all = "lowercase")]
pub enum ApiKey {
    Header(ApiKeyValue),
    Query(ApiKeyValue),
    Cookie(ApiKeyValue),
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ApiKeyValue {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub extensions: Option<Extensions>,
}

impl ApiKeyValue {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            description: None,
            extensions: Default::default(),
        }
    }

    pub fn with_description<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
            extensions: Default::default(),
        }
    }
}
