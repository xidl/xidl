use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::openapi::{
    Deprecated, RefOr, Required, Schema, builder, extensions::Extensions, set_value,
};

builder! {
    ParameterBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Parameter {
        pub name: String,
        #[serde(rename = "in")]
        pub parameter_in: ParameterIn,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        pub required: Required,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub deprecated: Option<Deprecated>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub schema: Option<RefOr<Schema>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub style: Option<ParameterStyle>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub explode: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub allow_reserved: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        example: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl Parameter {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            required: Required::True,
            ..Default::default()
        }
    }
}

impl ParameterBuilder {
    pub fn name<I: Into<String>>(mut self, name: I) -> Self {
        set_value!(self name name.into())
    }

    pub fn parameter_in(mut self, parameter_in: ParameterIn) -> Self {
        set_value!(self parameter_in parameter_in)
    }

    pub fn required(mut self, required: Required) -> Self {
        self.required = required;
        if self.parameter_in == ParameterIn::Path {
            self.required = Required::True;
        }
        self
    }

    pub fn description<S: Into<String>>(mut self, description: Option<S>) -> Self {
        set_value!(self description description.map(|description| description.into()))
    }

    pub fn deprecated(mut self, deprecated: Option<Deprecated>) -> Self {
        set_value!(self deprecated deprecated)
    }

    pub fn schema<I: Into<RefOr<Schema>>>(mut self, component: Option<I>) -> Self {
        set_value!(self schema component.map(|component| component.into()))
    }

    pub fn style(mut self, style: Option<ParameterStyle>) -> Self {
        set_value!(self style style)
    }

    pub fn explode(mut self, explode: Option<bool>) -> Self {
        set_value!(self explode explode)
    }

    pub fn allow_reserved(mut self, allow_reserved: Option<bool>) -> Self {
        set_value!(self allow_reserved allow_reserved)
    }

    pub fn example(mut self, example: Option<Value>) -> Self {
        set_value!(self example example)
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum ParameterIn {
    Query,
    #[default]
    Path,
    Header,
    Cookie,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ParameterStyle {
    Matrix,
    Label,
    Form,
    Simple,
    SpaceDelimited,
    PipeDelimited,
    DeepObject,
}
