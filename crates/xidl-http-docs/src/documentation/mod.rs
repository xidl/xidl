//! Source documentation joins typed HTTP operations without interpreting annotations.

mod metadata;
mod operation;
mod types;

use crate::error::Result;
use metadata::Metadata;
use operation::OperationDocumentation;
use serde_json::Value;
use std::collections::BTreeMap;
use types::TypeDocumentation;
use xidl_parser::{hir, rest_hir::RestHirDocument};
use xidlc::generate::openapi::OpenApiProjection;

#[derive(Default)]
pub(crate) struct DocumentationIndex {
    interfaces: BTreeMap<String, String>,
    methods: BTreeMap<String, MethodDocumentation>,
    types: TypeDocumentation,
}

struct MethodDocumentation {
    text: String,
    parameters: BTreeMap<String, Metadata>,
}

impl DocumentationIndex {
    pub fn collect(spec: &hir::Specification) -> Self {
        let mut index = Self::default();
        index.definitions(&spec.0, &[]);
        index.types.collect(&spec.0, &[]);
        index
    }

    fn definitions(&mut self, definitions: &[hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut path = scope.to_vec();
                    path.push(module.ident.clone());
                    self.definitions(&module.definition, &path);
                }
                hir::Definition::InterfaceDcl(interface) => {
                    let hir::InterfaceDclInner::InterfaceDef(definition) = &interface.decl else {
                        continue;
                    };
                    let mut path = scope.to_vec();
                    path.push(definition.header.ident.clone());
                    self.interfaces.insert(
                        path.join("."),
                        OpenApiProjection::documentation(&interface.annotations),
                    );
                    for export in definition.interface_body.iter().flat_map(|body| &body.0) {
                        if let hir::Export::OpDcl(operation) = export {
                            let mut name = path.clone();
                            name.push(operation.ident.clone());
                            self.methods.insert(
                                name.join("."),
                                MethodDocumentation {
                                    text: OpenApiProjection::documentation(&operation.annotations),
                                    parameters: operation
                                        .parameter
                                        .iter()
                                        .flat_map(|parameters| &parameters.0)
                                        .map(|parameter| {
                                            (
                                                parameter.declarator.0.clone(),
                                                Metadata {
                                                    annotations: parameter.annotations.clone(),
                                                    ty: Some(parameter.ty.clone()),
                                                    ..Metadata::default()
                                                },
                                            )
                                        })
                                        .collect(),
                                },
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn enrich_schemas(&self, schemas: &mut Value) -> Result<()> {
        self.types.enrich(schemas)
    }

    pub fn enrich(&self, id: &str, value: &mut Value, document: &RestHirDocument) -> Result<()> {
        let interface_name = id
            .rsplit_once('.')
            .map(|(interface, _)| interface)
            .unwrap_or_default();
        if let Some(description) = self
            .interfaces
            .get(interface_name)
            .filter(|description| !description.is_empty())
        {
            value["x-xidl-interface-description"] = description.clone().into();
        }
        let method = self.methods.get(id);
        if let Some(method) = method.filter(|method| !method.text.is_empty()) {
            value["description"] = method.text.clone().into();
        }
        {
            for interface in &document.interfaces {
                let prefix = interface
                    .module_path
                    .iter()
                    .chain(std::iter::once(&interface.name))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(".");
                if prefix != interface_name {
                    continue;
                }
                for operation in &interface.operations {
                    if format!("{prefix}.{}", operation.meta.name) != id {
                        continue;
                    }
                    OperationDocumentation {
                        source: method,
                        operation,
                    }
                    .enrich(value)?;
                }
            }
        }
        Ok(())
    }
}
