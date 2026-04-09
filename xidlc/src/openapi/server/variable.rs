use serde::{Deserialize, Serialize};

use crate::openapi::{builder, extensions::Extensions, set_value};

builder! {
    ServerVariableBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
    pub struct ServerVariable {
        #[serde(rename = "default")]
        pub default_value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
        pub enum_values: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl ServerVariableBuilder {
    pub fn default_value<S: Into<String>>(mut self, default_value: S) -> Self {
        set_value!(self default_value default_value.into())
    }

    pub fn description<S: Into<String>>(mut self, description: Option<S>) -> Self {
        set_value!(self description description.map(|description| description.into()))
    }

    pub fn enum_values<I: IntoIterator<Item = V>, V: Into<String>>(
        mut self,
        enum_values: Option<I>,
    ) -> Self {
        set_value!(self enum_values enum_values
            .map(|enum_values| enum_values.into_iter().map(|value| value.into()).collect()))
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }
}
