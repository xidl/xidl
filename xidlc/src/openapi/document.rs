use serde::{Deserialize, Serialize};

use super::{
    Components, ExternalDocs, Info, OpenApiVersion, Paths, SecurityRequirement, Server, Tag,
    builder, extensions::Extensions, path::PathsMap, set_value,
};

builder! {
    OpenApiBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct OpenApi {
        pub openapi: OpenApiVersion,
        pub info: Info,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub servers: Option<Vec<Server>>,
        pub paths: Paths,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub components: Option<Components>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub security: Option<Vec<SecurityRequirement>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tags: Option<Vec<Tag>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub external_docs: Option<ExternalDocs>,
        #[serde(rename = "$schema", default, skip_serializing_if = "String::is_empty")]
        pub schema: String,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl OpenApi {
    pub fn new<P: Into<Paths>>(info: Info, paths: P) -> Self {
        Self {
            info,
            paths: paths.into(),
            ..Default::default()
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn merge_from(mut self, other: OpenApi) -> OpenApi {
        self.merge(other);
        self
    }

    pub fn merge(&mut self, mut other: OpenApi) {
        merge_servers(&mut self.servers, &mut other.servers);
        if !other.paths.paths.is_empty() {
            self.paths.merge(other.paths);
        }
        merge_components(&mut self.components, &mut other.components);
        merge_vec(&mut self.security, &mut other.security);
        merge_vec(&mut self.tags, &mut other.tags);
    }

    pub fn nest<P: Into<String>, O: Into<OpenApi>>(self, path: P, other: O) -> Self {
        self.nest_with_path_composer(path, other, |base, path| format!("{base}{path}"))
    }

    pub fn nest_with_path_composer<
        P: Into<String>,
        O: Into<OpenApi>,
        F: Fn(&str, &str) -> String,
    >(
        mut self,
        path: P,
        other: O,
        composer: F,
    ) -> Self {
        let path: String = path.into();
        let mut other_api: OpenApi = other.into();
        let nested_paths = other_api
            .paths
            .paths
            .into_iter()
            .map(|(item_path, item)| (composer(&path, &item_path), item))
            .collect::<PathsMap<_, _>>();
        self.paths.paths.extend(nested_paths);
        other_api.paths.paths = PathsMap::new();
        self.merge_from(other_api)
    }
}

impl OpenApiBuilder {
    pub fn info<I: Into<Info>>(mut self, info: I) -> Self {
        set_value!(self info info.into())
    }

    pub fn servers<I: IntoIterator<Item = Server>>(mut self, servers: Option<I>) -> Self {
        set_value!(self servers servers.map(|servers| servers.into_iter().collect()))
    }

    pub fn paths<P: Into<Paths>>(mut self, paths: P) -> Self {
        set_value!(self paths paths.into())
    }

    pub fn components(mut self, components: Option<Components>) -> Self {
        set_value!(self components components)
    }

    pub fn security<I: IntoIterator<Item = SecurityRequirement>>(
        mut self,
        security: Option<I>,
    ) -> Self {
        set_value!(self security security.map(|security| security.into_iter().collect()))
    }

    pub fn tags<I: IntoIterator<Item = Tag>>(mut self, tags: Option<I>) -> Self {
        set_value!(self tags tags.map(|tags| tags.into_iter().collect()))
    }

    pub fn external_docs(mut self, external_docs: Option<ExternalDocs>) -> Self {
        set_value!(self external_docs external_docs)
    }

    pub fn schema<S: Into<String>>(mut self, schema: S) -> Self {
        set_value!(self schema schema.into())
    }
}

fn merge_servers(target: &mut Option<Vec<Server>>, source: &mut Option<Vec<Server>>) {
    if let Some(other_servers) = source {
        let servers = target.get_or_insert(Vec::new());
        other_servers.retain(|server| !servers.contains(server));
        servers.append(other_servers);
    }
}

fn merge_components(target: &mut Option<Components>, source: &mut Option<Components>) {
    if let Some(other_components) = source {
        let components = target.get_or_insert(Components::default());
        other_components
            .schemas
            .retain(|name, _| !components.schemas.contains_key(name));
        components.schemas.append(&mut other_components.schemas);
        other_components
            .responses
            .retain(|name, _| !components.responses.contains_key(name));
        components.responses.append(&mut other_components.responses);
        other_components
            .security_schemes
            .retain(|name, _| !components.security_schemes.contains_key(name));
        components
            .security_schemes
            .append(&mut other_components.security_schemes);
    }
}

fn merge_vec<T: PartialEq>(target: &mut Option<Vec<T>>, source: &mut Option<Vec<T>>) {
    if let Some(other_items) = source {
        let items = target.get_or_insert(Vec::new());
        other_items.retain(|item| !items.contains(item));
        items.append(other_items);
    }
}
