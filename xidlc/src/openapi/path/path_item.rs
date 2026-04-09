use serde::{Deserialize, Serialize};

use super::{HttpMethod, Operation, Parameter};
use crate::openapi::{Server, builder, extensions::Extensions, set_value};

builder! {
    PathItemBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct PathItem {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub servers: Option<Vec<Server>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub parameters: Option<Vec<Parameter>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub get: Option<Operation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub put: Option<Operation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub post: Option<Operation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub delete: Option<Operation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub options: Option<Operation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub head: Option<Operation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub patch: Option<Operation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub trace: Option<Operation>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl PathItem {
    pub fn new<O: Into<Operation>>(http_method: HttpMethod, operation: O) -> Self {
        let mut path_item = Self::default();
        path_item.set_operation(http_method, operation.into());
        path_item
    }

    pub fn from_http_methods<I: IntoIterator<Item = HttpMethod>, O: Into<Operation>>(
        http_methods: I,
        operation: O,
    ) -> Self {
        let mut path_item = Self::default();
        path_item.set_operations(http_methods, operation.into());
        path_item
    }

    pub fn merge_operations(&mut self, path_item: PathItem) {
        merge_if_missing(&mut self.get, path_item.get);
        merge_if_missing(&mut self.put, path_item.put);
        merge_if_missing(&mut self.post, path_item.post);
        merge_if_missing(&mut self.delete, path_item.delete);
        merge_if_missing(&mut self.options, path_item.options);
        merge_if_missing(&mut self.head, path_item.head);
        merge_if_missing(&mut self.patch, path_item.patch);
        merge_if_missing(&mut self.trace, path_item.trace);
    }

    pub(crate) fn operation(&self, http_method: HttpMethod) -> Option<&Operation> {
        match http_method {
            HttpMethod::Get => self.get.as_ref(),
            HttpMethod::Put => self.put.as_ref(),
            HttpMethod::Post => self.post.as_ref(),
            HttpMethod::Delete => self.delete.as_ref(),
            HttpMethod::Options => self.options.as_ref(),
            HttpMethod::Head => self.head.as_ref(),
            HttpMethod::Patch => self.patch.as_ref(),
            HttpMethod::Trace => self.trace.as_ref(),
        }
    }

    pub(crate) fn set_operations<I: IntoIterator<Item = HttpMethod>>(
        &mut self,
        http_methods: I,
        operation: Operation,
    ) {
        for method in http_methods {
            self.set_operation(method, operation.clone());
        }
    }

    fn set_operation(&mut self, http_method: HttpMethod, operation: Operation) {
        match http_method {
            HttpMethod::Get => self.get = Some(operation),
            HttpMethod::Put => self.put = Some(operation),
            HttpMethod::Post => self.post = Some(operation),
            HttpMethod::Delete => self.delete = Some(operation),
            HttpMethod::Options => self.options = Some(operation),
            HttpMethod::Head => self.head = Some(operation),
            HttpMethod::Patch => self.patch = Some(operation),
            HttpMethod::Trace => self.trace = Some(operation),
        }
    }
}

impl PathItemBuilder {
    pub fn operation<O: Into<Operation>>(mut self, http_method: HttpMethod, operation: O) -> Self {
        self.set_operation(http_method, operation.into());
        self
    }

    pub fn summary<S: Into<String>>(mut self, summary: Option<S>) -> Self {
        set_value!(self summary summary.map(|summary| summary.into()))
    }

    pub fn description<S: Into<String>>(mut self, description: Option<S>) -> Self {
        set_value!(self description description.map(|description| description.into()))
    }

    pub fn servers<I: IntoIterator<Item = Server>>(mut self, servers: Option<I>) -> Self {
        set_value!(self servers servers.map(|servers| servers.into_iter().collect()))
    }

    pub fn parameters<I: IntoIterator<Item = Parameter>>(mut self, parameters: Option<I>) -> Self {
        set_value!(self parameters parameters.map(|parameters| parameters.into_iter().collect()))
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }

    fn set_operation(&mut self, http_method: HttpMethod, operation: Operation) {
        match http_method {
            HttpMethod::Get => self.get = Some(operation),
            HttpMethod::Put => self.put = Some(operation),
            HttpMethod::Post => self.post = Some(operation),
            HttpMethod::Delete => self.delete = Some(operation),
            HttpMethod::Options => self.options = Some(operation),
            HttpMethod::Head => self.head = Some(operation),
            HttpMethod::Patch => self.patch = Some(operation),
            HttpMethod::Trace => self.trace = Some(operation),
        }
    }
}

fn merge_if_missing<T>(target: &mut Option<T>, source: Option<T>) {
    if target.is_none() {
        *target = source;
    }
}
