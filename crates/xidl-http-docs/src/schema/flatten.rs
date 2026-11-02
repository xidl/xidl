use crate::error::{DocumentationError, Result};
use crate::links::{Links, SchemaId};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct OptionalFlatten<'a> {
    pub schemas: &'a BTreeMap<SchemaId, Value>,
}

impl OptionalFlatten<'_> {
    pub fn apply(&self, origin: &SchemaId, schema: &mut Value) -> Result<()> {
        match schema {
            Value::Object(object) => {
                for value in object.values_mut() {
                    self.apply(origin, value)?;
                }
            }
            Value::Array(values) => {
                for value in values {
                    self.apply(origin, value)?;
                }
            }
            _ => {}
        }
        if schema["x-xidl-optional-flatten"] == true {
            let mut names = BTreeSet::new();
            self.names(origin, schema, (&mut BTreeSet::new(), &mut names))?;
            let absent = names
                .into_iter()
                .map(|name| json!({"required":[name]}))
                .collect::<Vec<_>>();
            let mut present = schema.clone();
            if let Some(object) = present.as_object_mut() {
                object.remove("x-xidl-optional-flatten");
            }
            *schema = json!({"x-xidl-optional-flatten":true,"anyOf":[present,{"type":"object","not":{"anyOf":absent}}]});
        }
        Ok(())
    }

    fn names(
        &self,
        origin: &SchemaId,
        schema: &Value,
        state: (&mut BTreeSet<SchemaId>, &mut BTreeSet<String>),
    ) -> Result<()> {
        let (visited, names) = state;
        names.extend(
            schema
                .get("properties")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|properties| properties.keys().cloned()),
        );
        if let Some(reference) = schema["$ref"].as_str() {
            let id = Links::resolve(origin, reference)?;
            if visited.insert(id.clone()) {
                let target = self.schemas.get(&id).ok_or_else(|| {
                    DocumentationError::Contract(format!(
                        "optional flattened group references missing type '{}::{}'",
                        id.unit, id.name
                    ))
                })?;
                self.names(&id, target, (visited, names))?;
            }
        }
        for keyword in ["allOf", "anyOf"] {
            for part in schema
                .get(keyword)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                self.names(origin, part, (visited, names))?;
            }
        }
        Ok(())
    }
}
