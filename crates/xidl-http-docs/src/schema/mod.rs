mod flatten;
mod position;
pub(crate) use position::SchemaPosition;

use crate::config::DocumentationConfig;
use crate::error::Result;
use crate::links::{Links, SchemaId};
use crate::page::ReferenceRow;
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) struct SchemaView<'a> {
    pub origin: &'a SchemaId,
    pub schemas: &'a BTreeMap<SchemaId, Value>,
    pub config: &'a DocumentationConfig,
}

impl SchemaView<'_> {
    pub fn normalize(schemas: &mut BTreeMap<SchemaId, Value>) -> Result<()> {
        let source = schemas.clone();
        for (id, schema) in schemas {
            flatten::OptionalFlatten { schemas: &source }.apply(id, schema)?;
        }
        Ok(())
    }

    pub fn fields(&self, schema: &Value) -> Result<Vec<ReferenceRow>> {
        let mut rows = Vec::new();
        self.collect_fields(schema, &mut Vec::new(), &mut rows)?;
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        rows.dedup_by(|a, b| a.name == b.name && a.details == b.details);
        Ok(rows)
    }

    fn collect_fields(
        &self,
        schema: &Value,
        stack: &mut Vec<SchemaId>,
        rows: &mut Vec<ReferenceRow>,
    ) -> Result<()> {
        if schema["x-xidl-optional-flatten"] == true {
            let start = rows.len();
            if let Some(group) = schema
                .get("anyOf")
                .and_then(Value::as_array)
                .and_then(|parts| parts.first())
            {
                self.collect_fields(group, stack, rows)?;
                for row in &mut rows[start..] {
                    row.required = false;
                }
            }
            return Ok(());
        }
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let id = Links::resolve(self.origin, reference)?;
            if !stack.contains(&id) {
                if let Some(target) = self.schemas.get(&id) {
                    stack.push(id.clone());
                    SchemaView {
                        origin: &id,
                        schemas: self.schemas,
                        config: self.config,
                    }
                    .collect_fields(target, stack, rows)?;
                    stack.pop();
                }
            }
        }
        for part in schema
            .get("allOf")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            self.collect_fields(part, stack, rows)?;
        }
        for (name, field) in schema
            .get("properties")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            let required = schema
                .get("required")
                .and_then(Value::as_array)
                .is_some_and(|required| required.iter().any(|value| value.as_str() == Some(name)));
            rows.push(self.row(name, "field", required, field)?);
        }
        Ok(())
    }

    pub fn row(
        &self,
        name: &str,
        location: &str,
        required: bool,
        schema: &Value,
    ) -> Result<ReferenceRow> {
        Ok(ReferenceRow {
            documentation_origin: self.origin.clone(),
            declared_default: schema["x-xidl-declared-default"]
                .as_str()
                .map(str::to_owned),
            constraints: schema
                .get("x-xidl-declared-constraints")
                .map(serde_json::to_string_pretty)
                .transpose()?
                .unwrap_or_default(),
            deprecated: schema["deprecated"].as_bool().unwrap_or(false),
            lifecycle: serde_json::to_string_pretty(&schema["x-xidl-deprecation"])?,
            name: name.into(),
            anchor: if location == "field" {
                format!("field-{}", Links::segment(name))
            } else {
                format!(
                    "parameter-{}-{}",
                    Links::segment(location),
                    Links::segment(name)
                )
            },
            location: location.into(),
            required,
            ty: Self::type_name(schema),
            description: schema
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("No description provided.")
                .into(),
            details: serde_json::to_string_pretty(schema)?,
            links: Links::definitions(self.origin, schema, self.schemas, self.config)?,
        })
    }

    pub fn type_name(schema: &Value) -> String {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            return reference
                .rsplit('/')
                .next()
                .unwrap_or(reference)
                .replace('.', "::");
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("array") => format!("array<{}>", Self::type_name(&schema["items"])),
            Some(value) => match schema.get("format").and_then(Value::as_str) {
                Some(format) => format!("{value} ({format})"),
                None => value.into(),
            },
            None if schema.get("allOf").is_some() => "intersection (see wire schema)".into(),
            None if schema.get("oneOf").is_some() => "union (see wire schema)".into(),
            _ => "see wire schema".into(),
        }
    }
}
