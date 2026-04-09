use serde::{Deserialize, Serialize};

use crate::openapi::{builder, extensions::Extensions, set_value};

builder! {
    ContactBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct Contact {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub email: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl Contact {
    pub fn new() -> Self {
        Default::default()
    }
}

impl ContactBuilder {
    pub fn name<S: Into<String>>(mut self, name: Option<S>) -> Self {
        set_value!(self name name.map(|name| name.into()))
    }

    pub fn url<S: Into<String>>(mut self, url: Option<S>) -> Self {
        set_value!(self url url.map(|url| url.into()))
    }

    pub fn email<S: Into<String>>(mut self, email: Option<S>) -> Self {
        set_value!(self email email.map(|email| email.into()))
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }
}
