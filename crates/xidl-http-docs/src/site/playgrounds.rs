use super::Site;
use crate::contract::Contract;
use crate::error::{DocumentationError, Result};
use crate::links::{Links, SchemaId};
use crate::markdown::Markdown;
use crate::schema::SchemaPosition;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

struct PublishedOpenApi<'a> {
    contract: &'a Contract,
    schemas: &'a BTreeMap<SchemaId, Value>,
}

impl Site<'_> {
    pub(super) fn playgrounds(&self, contracts: &[Contract]) -> Result<BTreeMap<String, String>> {
        contracts
            .iter()
            .filter(|contract| !contract.operations.is_empty())
            .map(|contract| {
                let document = PublishedOpenApi {
                    contract,
                    schemas: &self.schemas,
                }
                .document()?;
                Ok((
                    format!(
                        "public/{}",
                        Links::specification(&contract.unit, &contract.version)
                    ),
                    serde_json::to_string_pretty(&document)?,
                ))
            })
            .collect()
    }
}

impl PublishedOpenApi<'_> {
    fn document(&self) -> Result<Value> {
        let mut document = self.contract.openapi.clone();
        document["paths"] = json!({});
        document["webhooks"] = json!({});
        document["components"] = json!({"schemas":{},"securitySchemes":{}});
        let tags = self
            .contract
            .operations
            .values()
            .flat_map(|operation| {
                operation.value["tags"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
            })
            .collect::<BTreeSet<_>>();
        if let Some(published) = document["tags"].as_array_mut() {
            published.retain(|tag| tag["name"].as_str().is_some_and(|name| tags.contains(name)));
        }
        if let Some(description) = document["info"]["description"].as_str() {
            document["info"]["description"] = Markdown::safe_fragment(description.into()).into();
        }
        let mut pending = BTreeSet::new();
        for operation in self.contract.operations.values() {
            let mut value = operation.value.clone();
            self.select_version(&mut value);
            Self::rewrite(
                &self.contract.origin(),
                &mut value,
                &mut pending,
                SchemaPosition::Node,
            )?;
            for (method, path) in &operation.routes {
                document[&operation.publication][path][method.to_lowercase()] = value.clone();
            }
            for (name, scheme) in operation.security_schemes.as_object().into_iter().flatten() {
                document["components"]["securitySchemes"][name] = scheme.clone();
            }
        }
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop_first() {
            if !visited.insert(id.clone()) {
                continue;
            }
            let mut schema = self.schemas.get(&id).cloned().ok_or_else(|| {
                DocumentationError::Contract(format!(
                    "playground references missing published schema '{}::{}'",
                    id.unit, id.name
                ))
            })?;
            if let Some(object) = schema.as_object_mut() {
                object
                    .entry("title")
                    .or_insert_with(|| id.logical_name().into());
            }
            Self::rewrite(&id, &mut schema, &mut pending, SchemaPosition::Node)?;
            document["components"]["schemas"][Self::component_name(&id)] = schema;
        }
        Ok(document)
    }

    fn select_version(&self, operation: &mut Value) {
        let Some(version) = &self.contract.version_parameter else {
            return;
        };
        for parameter in operation["parameters"].as_array_mut().into_iter().flatten() {
            if parameter["in"] == version.location.as_str()
                && parameter["name"]
                    .as_str()
                    .is_some_and(|name| version.location.names_equal(name, &version.name))
            {
                parameter["schema"]["default"] = self.contract.wire_version.clone().into();
            }
        }
    }

    fn component_name(id: &SchemaId) -> String {
        [&id.unit, &id.version, &id.name]
            .into_iter()
            .map(|part| {
                part.bytes()
                    .map(|byte| {
                        if byte.is_ascii_alphanumeric() || byte == b'-' {
                            (byte as char).to_string()
                        } else {
                            format!("_{byte:02x}")
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    fn rewrite(
        origin: &SchemaId,
        value: &mut Value,
        pending: &mut BTreeSet<SchemaId>,
        position: SchemaPosition,
    ) -> Result<()> {
        if position == SchemaPosition::Literal {
            return Ok(());
        }
        match value {
            Value::Object(object) => {
                if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                    let id = Links::resolve(origin, reference)?;
                    object.insert(
                        "$ref".into(),
                        format!("#/components/{}/{}", "schemas", Self::component_name(&id)).into(),
                    );
                    pending.insert(id);
                }
                for (key, child) in object {
                    if position == SchemaPosition::Node && key == "description" && child.is_string()
                    {
                        if let Some(text) = child.as_str() {
                            *child = Markdown::safe_fragment(text.into()).into();
                        }
                    } else {
                        Self::rewrite(origin, child, pending, position.child(key))?;
                    }
                }
            }
            Value::Array(array) => {
                for child in array {
                    Self::rewrite(origin, child, pending, SchemaPosition::Node)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
