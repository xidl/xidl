use super::VersionLock;
use crate::error::{IdlcError, IdlcResult};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

type Reference = (String, String);

/// The generated operations and the constraints they reach over HTTP.
#[derive(Serialize)]
pub(super) struct WireContract {
    routes: WireProjection,
    webhooks: Vec<Vec<u8>>,
}

impl WireContract {
    /// Hashes published routes and webhook shapes, independent of generator names.
    pub(super) fn fingerprint(
        root: &Value,
        mut import_schema: impl FnMut(&str, &str) -> IdlcResult<Option<Value>>,
    ) -> IdlcResult<String> {
        let operation_fields = ["paths", "servers", "security"];
        let routes = Value::Object(
            operation_fields
                .into_iter()
                .filter_map(|key| root.get(key).map(|value| (key.to_owned(), value.clone())))
                .collect(),
        );
        let routes = WireProjection::project(root, routes, &mut import_schema)?;
        let mut webhooks = Vec::new();
        if let Some(items) = root.get("webhooks").and_then(Value::as_object) {
            for webhook in items.values() {
                let projection =
                    WireProjection::project(root, webhook.clone(), &mut import_schema)?;
                webhooks.push(serde_json::to_vec(&projection)?);
            }
            webhooks.sort();
        }
        let contract = Self { routes, webhooks };
        Ok(VersionLock::fingerprint(&serde_json::to_vec(&contract)?))
    }
}

#[derive(Serialize)]
struct WireProjection {
    operations: Value,
    references: Vec<Value>,
}

impl WireProjection {
    fn project(
        root: &Value,
        mut operations: Value,
        import_schema: &mut impl FnMut(&str, &str) -> IdlcResult<Option<Value>>,
    ) -> IdlcResult<Self> {
        let mut projection = ReferenceGraph {
            root,
            ids: BTreeMap::new(),
            pending: Vec::new(),
        };
        operations.sort_all_objects();
        projection.visit(&mut operations, "")?;
        let mut references = Vec::new();
        while let Some(reference) = projection.pending.get(references.len()).cloned() {
            let (file, pointer) = reference;
            let schema = if file.is_empty() {
                root.pointer(&pointer).cloned()
            } else {
                import_schema(&file, &pointer)?
            };
            let mut schema = schema.ok_or_else(|| ReferenceGraph::unresolved(&file, &pointer))?;
            schema.sort_all_objects();
            projection.visit(&mut schema, &file)?;
            references.push(schema);
        }
        Ok(Self {
            operations,
            references,
        })
    }
}

struct ReferenceGraph<'a> {
    root: &'a Value,
    ids: BTreeMap<Reference, usize>,
    pending: Vec<Reference>,
}

impl ReferenceGraph<'_> {
    fn unresolved(file: &str, pointer: &str) -> IdlcError {
        IdlcError::fmt(format!(
            "version lock: OpenAPI reference '{file}#{pointer}' does not resolve to a generated component"
        ))
    }

    fn reference_id(&mut self, from: &str, reference: &str) -> IdlcResult<usize> {
        let (file, pointer) = reference
            .split_once('#')
            .filter(|(_, pointer)| pointer.starts_with('/'))
            .ok_or_else(|| Self::unresolved(from, reference))?;
        let file = if file.is_empty() { from } else { file };
        let key = (file.to_owned(), pointer.to_owned());
        let id = self.ids.entry(key.clone()).or_insert_with(|| {
            let id = self.pending.len();
            self.pending.push(key);
            id
        });
        Ok(*id)
    }

    fn visit(&mut self, value: &mut Value, file: &str) -> IdlcResult<()> {
        match value {
            Value::Object(object) => {
                object.retain(|field, _| match field.as_str() {
                    "$comment" | "description" | "summary" | "title" | "deprecated"
                    | "operationId" | "tags" | "externalDocs" | "example" | "examples"
                    | "links" => false,
                    "x-protocol" | "x-websocket-subprotocol" => true,
                    _ => !field.starts_with("x-"),
                });

                // The generator uses a one-arm allOf to annotate a reference.
                if object.len() == 1
                    && let Some([reference]) = object
                        .get("allOf")
                        .and_then(Value::as_array)
                        .map(Vec::as_slice)
                    && reference.as_object().is_some_and(|map| {
                        map.len() == 1 && map.get("$ref").is_some_and(Value::is_string)
                    })
                {
                    *value = reference.clone();
                    return self.visit(value, file);
                }

                for (name, child) in object {
                    match name.as_str() {
                        "$ref" => {
                            if let Some(reference) = child.as_str() {
                                *child = Value::from(self.reference_id(file, reference)?);
                            }
                        }
                        // These maps carry wire names even when a key is named like metadata.
                        "properties" | "patternProperties" | "dependentSchemas" | "$defs"
                        | "schemas" | "paths" | "responses" | "headers" | "requestBodies"
                        | "content" | "encoding" | "callbacks" | "variables" | "pathItems" => {
                            if let Some(entries) = child.as_object_mut() {
                                for entry in entries.values_mut() {
                                    self.visit(entry, file)?;
                                }
                            }
                        }
                        "security" => self.security(child, file)?,
                        "required" | "enum" => {
                            if let Some(values) = child.as_array_mut() {
                                values.sort_by_cached_key(Value::to_string);
                            }
                        }
                        // These are literal wire data or discriminator values.
                        "default" | "const" | "discriminator" => {}
                        _ => self.visit(child, file)?,
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    self.visit(value, file)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn normalize_oauth_scopes(scheme: &mut Value) {
        if scheme.get("type").and_then(Value::as_str) != Some("oauth2") {
            return;
        }
        if let Some(flows) = scheme.get_mut("flows").and_then(Value::as_object_mut) {
            for flow in flows.values_mut() {
                if let Some(scopes) = flow.get_mut("scopes")
                    && let Some(entries) = scopes.as_object()
                {
                    let mut names: Vec<_> = entries.keys().cloned().collect();
                    names.sort_unstable();
                    *scopes = Value::from(names);
                }
            }
        }
    }

    fn security(&mut self, value: &mut Value, file: &str) -> IdlcResult<()> {
        let Some(requirements) = value.as_array_mut() else {
            return Ok(());
        };
        let root = self.root;
        let schemes = root
            .pointer("/components/securitySchemes")
            .and_then(Value::as_object);
        for requirement in requirements.iter_mut() {
            let Some(names) = requirement.as_object() else {
                continue;
            };
            let mut entries = Vec::new();
            for (name, scopes) in names {
                let mut scheme = schemes
                    .and_then(|schemes| schemes.get(name))
                    .cloned()
                    .ok_or_else(|| {
                        Self::unresolved(file, &format!("/components/securitySchemes/{name}"))
                    })?;
                Self::normalize_oauth_scopes(&mut scheme);
                scheme.sort_all_objects();
                self.visit(&mut scheme, file)?;
                let mut scopes = scopes.clone();
                if let Some(values) = scopes.as_array_mut() {
                    values.sort_by_cached_key(Value::to_string);
                }
                entries.push(Value::Array(vec![scheme, scopes]));
            }
            entries.sort_by_cached_key(Value::to_string);
            *requirement = Value::Array(entries);
        }
        requirements.sort_by_cached_key(Value::to_string);
        Ok(())
    }
}
