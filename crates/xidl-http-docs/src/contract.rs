use crate::config::DocumentationConfig;
use crate::documentation::DocumentationIndex;
use crate::error::{DocumentationError, Result};
use crate::idl::IdlSource;
use crate::links::{Links, SchemaId};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use xidlc::generate::openapi::OpenApiProjection;
use xidlc::jsonrpc::{CodegenInput, CompilationUnit, VersionInput};

pub(crate) struct Contract {
    pub unit: String,
    pub version: String,
    pub wire_version: String,
    pub title: String,
    pub idl: IdlSource,
    pub openapi: Value,
    pub version_parameter: Option<xidl_parser::hir::VersionParameter>,
    pub operations: BTreeMap<String, Operation>,
    pub schemas: BTreeMap<SchemaId, Value>,
}

pub(crate) struct Operation {
    pub name: String,
    pub publication: String,
    pub routes: Vec<(String, String)>,
    pub value: Value,
    pub security_schemes: Value,
}

impl Contract {
    pub fn from_input(
        unit: &CompilationUnit,
        input: &VersionInput,
        config: &DocumentationConfig,
    ) -> Result<Self> {
        let CodegenInput::RestHir(document) = &input.input else {
            return Err(DocumentationError::Contract(format!(
                "unit '{}' requires an HTTP projection; non-HTTP documentation is unsupported",
                unit.id
            )));
        };
        let version = input
            .version
            .clone()
            .unwrap_or_else(|| "unversioned".into());
        let props = std::collections::HashMap::from([("xidl_imports".into(), json!(unit.imports))]);
        let mut openapi = OpenApiProjection::render(document, &props).map_err(|error| {
            DocumentationError::Contract(format!("HTTP projection '{}': {error}", unit.id))
        })?;
        let documentation = DocumentationIndex::collect(&document.spec);
        documentation.enrich_schemas(&mut openapi["components"]["schemas"])?;
        for publication in ["paths", "webhooks"] {
            for item in openapi
                .get_mut(publication)
                .and_then(Value::as_object_mut)
                .into_iter()
                .flat_map(|items| items.values_mut())
            {
                for value in item
                    .as_object_mut()
                    .into_iter()
                    .flat_map(|item| item.values_mut())
                {
                    if let Some(id) = value
                        .get("operationId")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                    {
                        documentation.enrich(&id, value, document)?;
                    }
                }
            }
        }
        let mut contract = Self {
            unit: unit.id.clone(),
            version: version.clone(),
            wire_version: document
                .document
                .version
                .clone()
                .unwrap_or_else(|| version.clone()),
            title: document
                .document
                .package
                .clone()
                .unwrap_or_else(|| unit.id.clone()),
            idl: IdlSource::new(document, input.version.as_deref())?,
            openapi: openapi.clone(),
            version_parameter: document.document.version_parameter.clone(),
            operations: BTreeMap::new(),
            schemas: BTreeMap::new(),
        };
        for (name, schema) in openapi
            .pointer("/components/schemas")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            contract.schemas.insert(
                SchemaId {
                    unit: unit.id.clone(),
                    version: version.clone(),
                    name: name.clone(),
                },
                schema.clone(),
            );
        }
        for publication in ["paths", "webhooks"] {
            contract.collect_operations(&openapi, publication, config)?;
        }
        Ok(contract)
    }

    fn collect_operations(
        &mut self,
        openapi: &Value,
        publication: &str,
        config: &DocumentationConfig,
    ) -> Result<()> {
        for (route, item) in openapi
            .get(publication)
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            for (method, value) in item.as_object().into_iter().flatten() {
                if ![
                    "get", "put", "post", "patch", "delete", "options", "head", "trace",
                ]
                .contains(&method.as_str())
                {
                    continue;
                }
                let id = value
                    .get("operationId")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        DocumentationError::Contract(format!(
                            "HTTP operation '{route}' has no identity"
                        ))
                    })?;
                let name = id
                    .strip_prefix(&format!("{}.", self.version))
                    .unwrap_or(id)
                    .replace('.', "::");
                if config.excluded(&self.unit, &name) {
                    continue;
                }
                if let Some(existing) = self.operations.get_mut(&name) {
                    if existing.value != *value || existing.publication != publication {
                        return Err(DocumentationError::Contract(format!(
                            "conflicting operation identity '{}::{name}'",
                            self.unit
                        )));
                    }
                    existing.routes.push((method.to_uppercase(), route.clone()));
                } else {
                    self.operations.insert(
                        name.clone(),
                        Operation {
                            name,
                            publication: publication.into(),
                            routes: vec![(method.to_uppercase(), route.clone())],
                            value: value.clone(),
                            security_schemes: Self::security_schemes(openapi, value)?,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    pub fn origin(&self) -> SchemaId {
        SchemaId {
            unit: self.unit.clone(),
            version: self.version.clone(),
            name: String::new(),
        }
    }

    fn security_schemes(openapi: &Value, operation: &Value) -> Result<Value> {
        let mut schemes = serde_json::Map::new();
        for requirement in operation
            .get("security")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for name in requirement
                .as_object()
                .into_iter()
                .flat_map(|requirement| requirement.keys())
            {
                let scheme = openapi
                    .get("components")
                    .and_then(|components| components.get("securitySchemes"))
                    .and_then(|schemes| schemes.get(name))
                    .ok_or_else(|| {
                        DocumentationError::Contract(format!("missing security scheme '{name}'"))
                    })?;
                schemes.insert(name.clone(), scheme.clone());
            }
        }
        Ok(Value::Object(schemes))
    }

    pub fn operation_path(&self, name: &str) -> String {
        Links::operation(&self.unit, &self.version, name)
    }
}
