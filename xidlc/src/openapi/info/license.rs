use serde::{Deserialize, Serialize};

use crate::openapi::{builder, extensions::Extensions, set_value};

builder! {
    LicenseBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct License {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub identifier: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl License {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

impl LicenseBuilder {
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        set_value!(self name name.into())
    }

    pub fn url<S: Into<String>>(mut self, url: Option<S>) -> Self {
        set_value!(self url url.map(|url| url.into()))
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }

    pub fn identifier<S: Into<String>>(mut self, identifier: Option<S>) -> Self {
        set_value!(self identifier identifier.map(|identifier| identifier.into()))
    }
}
