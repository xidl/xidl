use super::Site;
use crate::contract::Contract;
use crate::error::Result;
use crate::links::Links;
use crate::page::{OperationPage, Page, PageContent, TypePage};
use crate::schema::SchemaView;
use serde_json::{Value, json};

impl Site<'_> {
    pub(super) fn references(&mut self, contracts: &[Contract]) -> Result<()> {
        for contract in contracts {
            for operation in contract.operations.values() {
                let origin = contract.origin();
                let view = SchemaView {
                    origin: &origin,
                    schemas: &self.schemas,
                    config: self.config,
                };
                let mut parameters = Vec::new();
                for parameter in operation
                    .value
                    .get("parameters")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let mut row = view.row(
                        parameter["name"].as_str().unwrap_or_default(),
                        parameter["in"].as_str().unwrap_or_default(),
                        parameter["required"].as_bool().unwrap_or(false),
                        &parameter["schema"],
                    )?;
                    row.description = parameter["description"]
                        .as_str()
                        .unwrap_or("No description provided.")
                        .into();
                    row.deprecated = parameter["deprecated"].as_bool().unwrap_or(false);
                    row.lifecycle = serde_json::to_string_pretty(&parameter["x-xidl-deprecation"])?;
                    parameters.push(row);
                }
                let page = OperationPage {
                    title: operation.name.clone(),
                    unit: contract.unit.clone(),
                    version: contract.version.clone(),
                    wire_version: contract.wire_version.clone(),
                    description: operation.value["description"]
                        .as_str()
                        .unwrap_or("No description provided.")
                        .into(),
                    interface_description: operation.value["x-xidl-interface-description"]
                        .as_str()
                        .unwrap_or_default()
                        .into(),
                    method: operation
                        .routes
                        .iter()
                        .map(|(method, _)| method.as_str())
                        .collect::<Vec<_>>()
                        .join(" / "),
                    route: operation
                        .routes
                        .iter()
                        .map(|(method, route)| format!("{method} {route}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    publication: operation.publication.clone(),
                    deprecated: operation.value["deprecated"].as_bool().unwrap_or(false),
                    parameters,
                    lifecycle: serde_json::to_string_pretty(
                        &operation.value["x-xidl-deprecation"],
                    )?,
                    request: serde_json::to_string_pretty(
                        &operation.value.get("requestBody").unwrap_or(&Value::Null),
                    )?,
                    responses: serde_json::to_string_pretty(&operation.value["responses"])?,
                    security: serde_json::to_string_pretty(
                        &json!({"requirements":operation.value.get("security").cloned().unwrap_or_else(|| json!([])),"schemes":operation.security_schemes}),
                    )?,
                    definitions: Links::definitions(
                        &origin,
                        &operation.value,
                        &self.schemas,
                        self.config,
                    )?,
                };
                self.insert(Page {
                    path: contract.operation_path(&operation.name),
                    title: page.title.clone(),
                    group: format!("{} / {}", contract.unit, contract.version),
                    markdown: self.renderer.template("operation.md", &page)?,
                    content: PageContent::Operation(Box::new(page)),
                })?;
            }
        }
        let mut pages = Vec::new();
        for (id, schema) in &self.schemas {
            let page = TypePage {
                title: id.logical_name(),
                unit: id.unit.clone(),
                version: id.version.clone(),
                description: schema["description"]
                    .as_str()
                    .unwrap_or("No description provided.")
                    .into(),
                deprecated: schema["deprecated"].as_bool().unwrap_or(false),
                lifecycle: serde_json::to_string_pretty(&schema["x-xidl-deprecation"])?,
                fields: SchemaView {
                    origin: id,
                    schemas: &self.schemas,
                    config: self.config,
                }
                .fields(schema)?,
                schema: serde_json::to_string_pretty(schema)?,
                definitions: Links::definitions(id, schema, &self.schemas, self.config)?,
                uses: self.uses.get(id).cloned().unwrap_or_default(),
            };
            pages.push(Page {
                path: id.path(),
                title: page.title.clone(),
                group: format!("{} / {}", id.unit, id.version),
                markdown: self.renderer.template("type.md", &page)?,
                content: PageContent::Type(Box::new(page)),
            });
        }
        for page in pages {
            self.insert(page)?;
        }
        Ok(())
    }
}
