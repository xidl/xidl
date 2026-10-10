use std::collections::BTreeSet;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::scope::resolve_canonical;
use xidl_parser::hir;

/// Names of the components emitted by this document, including later declarations.
pub(crate) struct SchemaNames(BTreeSet<String>);

impl SchemaNames {
    pub(crate) fn collect(spec: &hir::Specification) -> Self {
        let mut names = Self(BTreeSet::new());
        names.collect_definitions(&spec.0, &[]);
        names
    }

    fn insert(&mut self, module_path: &[String], ident: &str) {
        self.0.insert(
            module_path
                .iter()
                .map(String::as_str)
                .chain(std::iter::once(ident))
                .collect::<Vec<_>>()
                .join("::"),
        );
    }

    fn collect_definitions(&mut self, definitions: &[hir::Definition], module_path: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut nested = module_path.to_vec();
                    nested.push(module.ident.clone());
                    self.collect_definitions(&module.definition, &nested);
                }
                hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(constr))
                | hir::Definition::ConstrTypeDcl(constr) => {
                    self.collect_constructed(constr, module_path)
                }
                hir::Definition::TypeDcl(hir::TypeDcl::TypedefDcl(alias)) => {
                    for decl in &alias.decl {
                        self.insert(module_path, &super::naming::declarator_name(decl));
                    }
                    if let hir::TypedefType::ConstrTypeDcl(constr) = &alias.ty {
                        self.collect_constructed(constr, module_path);
                    }
                }
                hir::Definition::ExceptDcl(exception) => self.insert(module_path, &exception.ident),
                _ => {}
            }
        }
    }

    fn collect_constructed(&mut self, constr: &hir::ConstrTypeDcl, module_path: &[String]) {
        let ident = match constr {
            hir::ConstrTypeDcl::StructDcl(def) => &def.ident,
            hir::ConstrTypeDcl::EnumDcl(def) => &def.ident,
            hir::ConstrTypeDcl::UnionDef(def) => &def.ident,
            hir::ConstrTypeDcl::BitsetDcl(def) => &def.ident,
            hir::ConstrTypeDcl::BitmaskDcl(def) => &def.ident,
            hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {
                return;
            }
        };
        self.insert(module_path, ident);
    }
}

/// Declaration scope used while rendering OpenAPI schemas.
#[derive(Clone, Copy)]
pub(crate) struct SchemaScope<'a> {
    names: &'a SchemaNames,
    pub(crate) module_path: &'a [String],
}

impl<'a> SchemaScope<'a> {
    pub(crate) fn new(names: &'a SchemaNames, module_path: &'a [String]) -> Self {
        Self { names, module_path }
    }

    pub(crate) fn in_module(self, module_path: &'a [String]) -> Self {
        Self {
            module_path,
            ..self
        }
    }

    pub(crate) fn resolve(self, name: &hir::ScopedName) -> IdlcResult<String> {
        resolve_canonical(self.module_path, name, |key| self.names.0.contains(key))
            .map(|name| name.replace("::", "."))
            .ok_or_else(|| {
                IdlcError::rpc(format!(
                    "OpenAPI type '{}' does not resolve in scope '{}'",
                    name.name.join("::"),
                    self.module_path.join("::")
                ))
            })
    }
}
