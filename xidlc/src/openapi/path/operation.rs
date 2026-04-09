use serde::{Deserialize, Serialize};

use super::Parameter;
use crate::openapi::{
    Deprecated, ExternalDocs, RefOr, Response, Responses, Server, builder, extensions::Extensions,
    request_body::RequestBody, security::SecurityRequirement, set_value,
};

builder! {
    OperationBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Operation {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tags: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub operation_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub external_docs: Option<ExternalDocs>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub parameters: Option<Vec<Parameter>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub request_body: Option<RequestBody>,
        pub responses: Responses,
        #[allow(missing_docs)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub callbacks: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub deprecated: Option<Deprecated>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub security: Option<Vec<SecurityRequirement>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub servers: Option<Vec<Server>>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl Operation {
    pub fn new() -> Self {
        Default::default()
    }
}

impl OperationBuilder {
    pub fn tags<I: IntoIterator<Item = V>, V: Into<String>>(mut self, tags: Option<I>) -> Self {
        set_value!(self tags tags.map(|tags| tags.into_iter().map(Into::into).collect()))
    }

    pub fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.get_or_insert_with(Vec::new).push(tag.into());
        self
    }

    pub fn summary<S: Into<String>>(mut self, summary: Option<S>) -> Self {
        set_value!(self summary summary.map(|summary| summary.into()))
    }

    pub fn description<S: Into<String>>(mut self, description: Option<S>) -> Self {
        set_value!(self description description.map(|description| description.into()))
    }

    pub fn operation_id<S: Into<String>>(mut self, operation_id: Option<S>) -> Self {
        set_value!(self operation_id operation_id.map(|operation_id| operation_id.into()))
    }

    pub fn parameters<I: IntoIterator<Item = P>, P: Into<Parameter>>(
        mut self,
        parameters: Option<I>,
    ) -> Self {
        self.parameters = parameters.map(|parameters| {
            let mut merged = self.parameters.unwrap_or_default();
            merged.extend(parameters.into_iter().map(Into::into));
            merged
        });
        self
    }

    pub fn parameter<P: Into<Parameter>>(mut self, parameter: P) -> Self {
        self.parameters
            .get_or_insert_with(Vec::new)
            .push(parameter.into());
        self
    }

    pub fn request_body(mut self, request_body: Option<RequestBody>) -> Self {
        set_value!(self request_body request_body)
    }

    pub fn responses<R: Into<Responses>>(mut self, responses: R) -> Self {
        set_value!(self responses responses.into())
    }

    pub fn response<S: Into<String>, R: Into<RefOr<Response>>>(
        mut self,
        code: S,
        response: R,
    ) -> Self {
        self.responses
            .responses
            .insert(code.into(), response.into());
        self
    }

    pub fn deprecated(mut self, deprecated: Option<Deprecated>) -> Self {
        set_value!(self deprecated deprecated)
    }

    pub fn securities<I: IntoIterator<Item = SecurityRequirement>>(
        mut self,
        securities: Option<I>,
    ) -> Self {
        set_value!(self security securities.map(|securities| securities.into_iter().collect()))
    }

    pub fn security(mut self, security: SecurityRequirement) -> Self {
        self.security.get_or_insert_with(Vec::new).push(security);
        self
    }

    pub fn servers<I: IntoIterator<Item = Server>>(mut self, servers: Option<I>) -> Self {
        set_value!(self servers servers.map(|servers| servers.into_iter().collect()))
    }

    pub fn server(mut self, server: Server) -> Self {
        self.servers.get_or_insert_with(Vec::new).push(server);
        self
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }
}
