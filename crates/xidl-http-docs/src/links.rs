use crate::config::DocumentationConfig;
use crate::error::{DocumentationError, Result};
use crate::page::PageLink;
use crate::schema::SchemaPosition;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SchemaId {
    pub unit: String,
    pub version: String,
    pub name: String,
}

impl SchemaId {
    pub fn logical_name(&self) -> String {
        self.name
            .strip_prefix(&format!("{}.", self.version))
            .unwrap_or(&self.name)
            .replace('.', "::")
    }

    pub fn path(&self) -> String {
        format!(
            "api/{}/{}/types/{}.md",
            Links::segment(&self.unit),
            Links::segment(&self.version),
            Links::segment(&self.logical_name())
        )
    }
}

pub(crate) struct Links;

impl Links {
    /// Browser route registered by the official Scalar Starlight integration.
    pub fn playground(unit: &str, version: &str) -> String {
        format!(
            "playground/{}/{}/",
            Self::segment(unit),
            Self::segment(version)
        )
    }

    /// Self-contained published HTTP document for one logical version.
    pub fn specification(unit: &str, version: &str) -> String {
        format!(
            "openapi/{}/{}.json",
            Self::segment(unit),
            Self::segment(version)
        )
    }

    pub fn slug(path: &str) -> &str {
        path.strip_suffix(".md").unwrap_or(path)
    }

    pub fn site_path(path: &str) -> String {
        let slug = Self::slug(path);
        match slug {
            "index" => String::new(),
            _ => format!("{}/", slug.strip_suffix("/index").unwrap_or(slug)),
        }
    }

    pub fn segment(value: &str) -> String {
        value
            .bytes()
            .map(|byte| {
                if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' {
                    (byte as char).to_string()
                } else {
                    format!("~{byte:02x}")
                }
            })
            .collect()
    }

    pub fn operation(unit: &str, version: &str, name: &str) -> String {
        format!(
            "api/{}/{}/operations/{}.md",
            Self::segment(unit),
            Self::segment(version),
            Self::segment(name)
        )
    }

    pub fn references(value: &Value) -> BTreeSet<String> {
        let mut refs = BTreeSet::new();
        Self::visit(value, &mut refs, SchemaPosition::Node);
        refs
    }

    fn visit(value: &Value, refs: &mut BTreeSet<String>, position: SchemaPosition) {
        if position == SchemaPosition::Literal {
            return;
        }
        match value {
            Value::Object(object) => {
                if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                    refs.insert(reference.to_owned());
                }
                for (key, child) in object {
                    Self::visit(child, refs, position.child(key));
                }
            }
            Value::Array(array) => {
                for child in array {
                    Self::visit(child, refs, SchemaPosition::Node);
                }
            }
            _ => {}
        }
    }

    pub fn resolve(origin: &SchemaId, reference: &str) -> Result<SchemaId> {
        let Some((document, name)) = reference.split_once("#/components/schemas/") else {
            return Err(DocumentationError::Contract(format!(
                "unsupported schema reference '{reference}'"
            )));
        };
        let name = name.replace("~1", "/").replace("~0", "~");
        let (unit, version) = if document.is_empty() {
            (origin.unit.clone(), origin.version.clone())
        } else {
            let stem = document
                .strip_prefix("openapi_")
                .and_then(|value| value.strip_suffix(".json"))
                .ok_or_else(|| {
                    DocumentationError::Contract(format!(
                        "unknown imported schema document '{document}'"
                    ))
                })?;
            match stem.rsplit_once('.') {
                Some((unit, version)) => (unit.to_owned(), version.to_owned()),
                None => (stem.to_owned(), "unversioned".into()),
            }
        };
        Ok(SchemaId {
            unit,
            version,
            name,
        })
    }

    pub fn definitions(
        origin: &SchemaId,
        value: &Value,
        schemas: &BTreeMap<SchemaId, Value>,
        config: &DocumentationConfig,
    ) -> Result<Vec<PageLink>> {
        Self::references(value)
            .iter()
            .map(|reference| {
                let id = Self::resolve(origin, reference)?;
                if !schemas.contains_key(&id) {
                    return Err(DocumentationError::Contract(format!(
                        "missing or excluded schema reference '{reference}' in {}",
                        origin.unit
                    )));
                }
                Ok(PageLink {
                    title: format!("{}::{}", id.unit, id.logical_name()),
                    url: config.url(&id.path()),
                })
            })
            .collect()
    }
}
