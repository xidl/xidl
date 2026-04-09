use serde::{Deserialize, Serialize};

use super::{Contact, License};
use crate::openapi::{builder, extensions::Extensions, set_value};

builder! {
    InfoBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct Info {
        pub title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub terms_of_service: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub contact: Option<Contact>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub license: Option<License>,
        pub version: String,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl Info {
    pub fn new<S: Into<String>>(title: S, version: S) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            ..Default::default()
        }
    }
}

impl InfoBuilder {
    pub fn title<I: Into<String>>(mut self, title: I) -> Self {
        set_value!(self title title.into())
    }

    pub fn version<I: Into<String>>(mut self, version: I) -> Self {
        set_value!(self version version.into())
    }

    pub fn description<S: Into<String>>(mut self, description: Option<S>) -> Self {
        set_value!(self description description.map(|description| description.into()))
    }

    pub fn terms_of_service<S: Into<String>>(mut self, terms_of_service: Option<S>) -> Self {
        set_value!(self terms_of_service terms_of_service.map(|terms_of_service| terms_of_service.into()))
    }

    pub fn contact(mut self, contact: Option<Contact>) -> Self {
        set_value!(self contact contact)
    }

    pub fn license(mut self, license: Option<License>) -> Self {
        set_value!(self license license)
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }
}
