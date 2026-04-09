use std::collections::BTreeMap;

use crate::openapi::extensions::Extensions;
use serde::{Deserialize, Serialize};

use super::security_flow::{AuthorizationCode, ClientCredentials, Implicit, Password};

#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct OAuth2 {
    pub flows: BTreeMap<String, Flow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub extensions: Option<Extensions>,
}

impl OAuth2 {
    pub fn new<I: IntoIterator<Item = Flow>>(flows: I) -> Self {
        Self {
            flows: BTreeMap::from_iter(
                flows
                    .into_iter()
                    .map(|flow| (flow.get_type_as_str().to_string(), flow)),
            ),
            extensions: None,
            description: None,
        }
    }

    pub fn with_description<I: IntoIterator<Item = Flow>, S: Into<String>>(
        flows: I,
        description: S,
    ) -> Self {
        Self {
            flows: BTreeMap::from_iter(
                flows
                    .into_iter()
                    .map(|flow| (flow.get_type_as_str().to_string(), flow)),
            ),
            extensions: None,
            description: Some(description.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Flow {
    Implicit(Implicit),
    Password(Password),
    ClientCredentials(ClientCredentials),
    AuthorizationCode(AuthorizationCode),
}

impl Flow {
    fn get_type_as_str(&self) -> &str {
        match self {
            Self::Implicit(_) => "implicit",
            Self::Password(_) => "password",
            Self::ClientCredentials(_) => "clientCredentials",
            Self::AuthorizationCode(_) => "authorizationCode",
        }
    }
}
