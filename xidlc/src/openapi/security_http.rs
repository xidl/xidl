use crate::openapi::extensions::Extensions;
use serde::{Deserialize, Serialize};

use crate::openapi::builder;

builder! {
    HttpBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct Http {
        pub scheme: HttpAuthScheme,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bearer_format: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl Http {
    pub fn new(scheme: HttpAuthScheme) -> Self {
        Self {
            scheme,
            bearer_format: None,
            description: None,
            extensions: Default::default(),
        }
    }
}

impl HttpBuilder {
    pub fn scheme(mut self, scheme: HttpAuthScheme) -> Self {
        self.scheme = scheme;
        self
    }

    pub fn bearer_format<S: Into<String>>(mut self, bearer_format: S) -> Self {
        if self.scheme == HttpAuthScheme::Bearer {
            self.bearer_format = Some(bearer_format.into());
        }
        self
    }

    pub fn description<S: Into<String>>(mut self, description: Option<S>) -> Self {
        self.description = description.map(Into::into);
        self
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[allow(missing_docs)]
pub enum HttpAuthScheme {
    #[default]
    Basic,
    Bearer,
    Digest,
    Hoba,
    Mutual,
    Negotiate,
    OAuth,
    #[serde(rename = "scram-sha-1")]
    ScramSha1,
    #[serde(rename = "scram-sha-256")]
    ScramSha256,
    Vapid,
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenIdConnect {
    pub open_id_connect_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub extensions: Option<Extensions>,
}

impl OpenIdConnect {
    pub fn new<S: Into<String>>(open_id_connect_url: S) -> Self {
        Self {
            open_id_connect_url: open_id_connect_url.into(),
            description: None,
            extensions: Default::default(),
        }
    }

    pub fn with_description<S: Into<String>>(open_id_connect_url: S, description: S) -> Self {
        Self {
            open_id_connect_url: open_id_connect_url.into(),
            description: Some(description.into()),
            extensions: Default::default(),
        }
    }
}
