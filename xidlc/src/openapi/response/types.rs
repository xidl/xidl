use std::collections::BTreeMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::IntoResponses;
use crate::openapi::{Ref, RefOr};

use super::super::{
    Content, builder, extensions::Extensions, header::Header, link::Link, set_value,
};

builder! {
    ResponsesBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Responses {
        #[serde(flatten)]
        pub responses: BTreeMap<String, RefOr<Response>>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl Responses {
    pub fn new() -> Self {
        Default::default()
    }
}

impl ResponsesBuilder {
    pub fn response<S: Into<String>, R: Into<RefOr<Response>>>(
        mut self,
        code: S,
        response: R,
    ) -> Self {
        self.responses.insert(code.into(), response.into());
        self
    }

    pub fn responses_from_iter<
        I: IntoIterator<Item = (C, R)>,
        C: Into<String>,
        R: Into<RefOr<Response>>,
    >(
        mut self,
        iter: I,
    ) -> Self {
        self.responses.extend(
            iter.into_iter()
                .map(|(code, response)| (code.into(), response.into())),
        );
        self
    }

    pub fn responses_from_into_responses<I: IntoResponses>(mut self) -> Self {
        self.responses.extend(I::responses());
        self
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }
}

impl From<Responses> for BTreeMap<String, RefOr<Response>> {
    fn from(responses: Responses) -> Self {
        responses.responses
    }
}

impl<C, R> FromIterator<(C, R)> for Responses
where
    C: Into<String>,
    R: Into<RefOr<Response>>,
{
    fn from_iter<T: IntoIterator<Item = (C, R)>>(iter: T) -> Self {
        Self {
            responses: BTreeMap::from_iter(
                iter.into_iter()
                    .map(|(code, response)| (code.into(), response.into())),
            ),
            ..Default::default()
        }
    }
}

builder! {
    ResponseBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        pub description: String,
        #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
        pub headers: BTreeMap<String, Header>,
        #[serde(skip_serializing_if = "IndexMap::is_empty", default)]
        pub content: IndexMap<String, Content>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
        #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
        pub links: BTreeMap<String, RefOr<Link>>,
    }
}

impl Response {
    pub fn new<S: Into<String>>(description: S) -> Self {
        Self {
            description: description.into(),
            ..Default::default()
        }
    }
}

impl ResponseBuilder {
    pub fn description<I: Into<String>>(mut self, description: I) -> Self {
        set_value!(self description description.into())
    }

    pub fn content<S: Into<String>>(mut self, content_type: S, content: Content) -> Self {
        self.content.insert(content_type.into(), content);
        self
    }

    pub fn header<S: Into<String>>(mut self, name: S, header: Header) -> Self {
        self.headers.insert(name.into(), header);
        self
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }

    pub fn link<S: Into<String>, L: Into<RefOr<Link>>>(mut self, name: S, link: L) -> Self {
        self.links.insert(name.into(), link.into());
        self
    }
}

impl From<ResponseBuilder> for RefOr<Response> {
    fn from(builder: ResponseBuilder) -> Self {
        Self::T(builder.build())
    }
}

impl From<Ref> for RefOr<Response> {
    fn from(r: Ref) -> Self {
        Self::Ref(r)
    }
}
