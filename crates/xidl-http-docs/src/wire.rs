//! Comparison surfaces preserve business names and omit documentation metadata.

use serde_json::{Value, json};
use xidl_parser::hir::{HttpParameterLocation, VersionParameter};

pub(crate) struct WireSurface;

impl WireSurface {
    pub fn normalize(value: &Value) -> Value {
        Self::node(value.clone(), false)
    }

    fn node(mut value: Value, names: bool) -> Value {
        match &mut value {
            Value::Object(object) => {
                if !names {
                    for key in [
                        "description",
                        "summary",
                        "deprecated",
                        "operationId",
                        "tags",
                        "externalDocs",
                        "x-xidl-interface-description",
                        "x-xidl-deprecation",
                    ] {
                        object.remove(key);
                    }
                }
                for (key, child) in object.iter_mut() {
                    if !names
                        && [
                            "enum", "const", "default", "example", "examples", "security",
                        ]
                        .contains(&key.as_str())
                    {
                        continue;
                    }
                    let mapping = !names
                        && [
                            "properties",
                            "patternProperties",
                            "$defs",
                            "definitions",
                            "schemas",
                            "headers",
                            "responses",
                            "content",
                            "securitySchemes",
                        ]
                        .contains(&key.as_str());
                    *child = Self::node(child.take(), mapping);
                }
            }
            Value::Array(array) => {
                for child in array {
                    *child = Self::node(child.take(), false);
                }
            }
            _ => {}
        }
        value
    }

    pub fn operation(value: &Value, version: Option<&VersionParameter>) -> Value {
        let mut value = value.clone();
        if let Some(version) = version {
            for parameter in value
                .get_mut("parameters")
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
            {
                if parameter["in"].as_str() == Some(version.location.as_str())
                    && parameter["name"]
                        .as_str()
                        .is_some_and(|name| version.location.names_equal(name, &version.name))
                {
                    parameter["schema"]["enum"] = json!(["<selected API version>"]);
                }
            }
            if version.location == HttpParameterLocation::Header {
                for response in value
                    .get_mut("responses")
                    .and_then(Value::as_object_mut)
                    .into_iter()
                    .flat_map(|responses| responses.values_mut())
                {
                    for (name, header) in response
                        .get_mut("headers")
                        .and_then(Value::as_object_mut)
                        .into_iter()
                        .flatten()
                    {
                        if version.location.names_equal(name, &version.name) {
                            header["schema"]["enum"] = json!(["<selected API version>"]);
                        }
                    }
                }
            }
        }
        value
    }

    pub fn local_references(value: &mut Value, version: &str) {
        match value {
            Value::Object(object) => {
                if let Some(reference) = object.get_mut("$ref") {
                    if let Some(name) = reference.as_str().and_then(|reference| {
                        reference.strip_prefix(&format!("#/components/schemas/{version}."))
                    }) {
                        *reference = format!("#/components/schemas/{name}").into();
                    }
                }
                for child in object.values_mut() {
                    Self::local_references(child, version);
                }
            }
            Value::Array(array) => {
                for child in array {
                    Self::local_references(child, version);
                }
            }
            _ => {}
        }
    }
}
