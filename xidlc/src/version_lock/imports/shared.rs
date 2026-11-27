//! Declarations and their local references in one unversioned import.

use crate::error::IdlcResult;
use crate::fmt::format_idl_source;
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::hir::printer::IdlPrinter;
use xidl_parser::hir::{
    DeclarationNames, Definition, Export, InterfaceDcl, InterfaceDclInner, ScopedName,
    Specification, VisitScopedNames,
};

pub(super) struct SharedImport {
    pub(super) path: String,
    pub(super) bindings: BTreeSet<String>,
    spec: Specification,
    pub(super) index: DeclarationIndex,
}

impl SharedImport {
    pub(super) fn new(path: String, bindings: BTreeSet<String>, mut spec: Specification) -> Self {
        let mut index = DeclarationIndex {
            names: DeclarationNames::collect(&spec.0),
            ..Default::default()
        };
        index.record(&mut spec.0, &mut Vec::new(), None);
        Self {
            path,
            bindings,
            spec,
            index,
        }
    }

    pub(super) fn render(&self, selected: &BTreeSet<usize>) -> IdlcResult<String> {
        let mut spec = self.spec.clone();
        Self::retain(&mut spec.0, selected, &mut 0);
        format_idl_source(&IdlPrinter::render(&spec))
    }

    fn retain(definitions: &mut Vec<Definition>, selected: &BTreeSet<usize>, next: &mut usize) {
        definitions.retain_mut(|definition| match definition {
            Definition::Pragma(_) => true,
            Definition::ModuleDcl(module) => {
                let keep = Self::keep_declaration(selected, next);
                Self::retain(&mut module.definition, selected, next);
                keep && !module.definition.is_empty()
            }
            Definition::InterfaceDcl(interface) => {
                let keep = Self::keep_declaration(selected, next);
                if let InterfaceDclInner::InterfaceDef(definition) = &mut interface.decl
                    && let Some(body) = &mut definition.interface_body
                {
                    body.0.retain(|_| Self::keep_declaration(selected, next));
                }
                keep
            }
            _ => Self::keep_declaration(selected, next),
        });
        if definitions
            .iter()
            .all(|definition| matches!(definition, Definition::Pragma(_)))
        {
            definitions.clear();
        }
    }

    fn keep_declaration(selected: &BTreeSet<usize>, next: &mut usize) -> bool {
        let id = *next;
        *next += 1;
        selected.contains(&id)
    }
}

#[derive(Default)]
pub(super) struct DeclarationIndex {
    names: DeclarationNames,
    pub(super) by_name: BTreeMap<String, Vec<usize>>,
    pub(super) declarations: Vec<IndexedDeclaration>,
}

pub(super) struct IndexedDeclaration {
    pub(super) scope: Vec<String>,
    pub(super) references: Vec<ScopedName>,
    pub(super) container: Option<usize>,
}

impl DeclarationIndex {
    fn record(
        &mut self,
        definitions: &mut [Definition],
        scope: &mut Vec<String>,
        container: Option<usize>,
    ) {
        for definition in definitions {
            match definition {
                Definition::Pragma(_) => {}
                Definition::ModuleDcl(module) => {
                    let id = self.declarations.len();
                    let mut references = Vec::new();
                    module
                        .annotations
                        .visit_scoped_names_with_annotations(&mut |name| {
                            references.push(name.clone());
                        });
                    self.declarations.push(IndexedDeclaration {
                        scope: scope.clone(),
                        references,
                        container,
                    });
                    scope.push(module.ident.clone());
                    self.record(&mut module.definition, scope, Some(id));
                    scope.pop();
                }
                Definition::InterfaceDcl(interface) => {
                    self.record_interface(interface, scope, container);
                }
                _ => {
                    let id = self.declarations.len();
                    self.record_names(definition, scope, id);
                    let mut references = Vec::new();
                    definition.visit_scoped_names_with_annotations(&mut |name| {
                        references.push(name.clone())
                    });
                    self.declarations.push(IndexedDeclaration {
                        scope: scope.clone(),
                        references,
                        container,
                    });
                }
            }
        }
    }

    fn record_interface(
        &mut self,
        interface: &mut InterfaceDcl,
        scope: &mut Vec<String>,
        container: Option<usize>,
    ) {
        let id = self.declarations.len();
        let (name, parents, body) = match &mut interface.decl {
            InterfaceDclInner::InterfaceForwardDcl(forward) => (&forward.ident, None, None),
            InterfaceDclInner::InterfaceDef(definition) => (
                &definition.header.ident,
                definition.header.parent.as_ref(),
                definition.interface_body.as_mut(),
            ),
        };
        let mut references: Vec<_> = parents
            .into_iter()
            .flat_map(|parents| &parents.0)
            .map(|name| name.0.clone())
            .collect();
        interface
            .annotations
            .visit_scoped_names_with_annotations(&mut |name| {
                references.push(name.clone());
            });
        self.declarations.push(IndexedDeclaration {
            scope: scope.clone(),
            references,
            container,
        });
        scope.push(name.clone());
        if let Some(body) = body {
            for export in &mut body.0 {
                let export_id = self.declarations.len();
                let definition = match export {
                    Export::TypeDcl(value) => Some(Definition::TypeDcl(value.clone())),
                    Export::ConstDcl(value) => Some(Definition::ConstDcl(value.clone())),
                    Export::ExceptDcl(value) => Some(Definition::ExceptDcl(value.clone())),
                    _ => None,
                };
                if let Some(definition) = definition {
                    self.record_names(&definition, scope, export_id);
                }
                let mut references = Vec::new();
                export
                    .visit_scoped_names_with_annotations(&mut |name| references.push(name.clone()));
                self.declarations.push(IndexedDeclaration {
                    scope: scope.clone(),
                    references,
                    container: Some(id),
                });
            }
        }
        // Referencing the interface itself keeps its complete callable contract.
        self.by_name
            .entry(scope.join("::"))
            .or_default()
            .extend(id..self.declarations.len());
        scope.pop();
    }

    fn record_names(&mut self, definition: &Definition, scope: &[String], id: usize) {
        DeclarationNames::visit_declarations(
            std::slice::from_ref(definition),
            &scope.join("::"),
            &mut |name, _| self.by_name.entry(name.to_string()).or_default().push(id),
        );
    }

    pub(super) fn resolve(&self, scope: &[String], name: &ScopedName) -> Option<&[usize]> {
        let resolved = self.names.resolve(scope, name)?;
        self.by_name
            .get(&resolved.name.join("::"))
            .map(Vec::as_slice)
    }
}
