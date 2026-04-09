use std::{collections::BTreeMap, iter};

use serde::{Deserialize, Serialize};

use super::ServerVariable;
use crate::openapi::{builder, extensions::Extensions, set_value};

builder! {
    ServerBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct Server {
        pub url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub variables: Option<BTreeMap<String, ServerVariable>>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl Server {
    pub fn new<S: Into<String>>(url: S) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }
}

impl ServerBuilder {
    pub fn url<U: Into<String>>(mut self, url: U) -> Self {
        set_value!(self url url.into())
    }

    pub fn description<S: Into<String>>(mut self, description: Option<S>) -> Self {
        set_value!(self description description.map(|description| description.into()))
    }

    pub fn parameter<N: Into<String>, V: Into<ServerVariable>>(
        mut self,
        name: N,
        variable: V,
    ) -> Self {
        match self.variables {
            Some(ref mut variables) => {
                variables.insert(name.into(), variable.into());
            }
            None => {
                self.variables = Some(BTreeMap::from_iter(iter::once((
                    name.into(),
                    variable.into(),
                ))))
            }
        }
        self
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }
}
