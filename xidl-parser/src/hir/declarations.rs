//! Canonical declaration paths shared by semantic analysis and version projection.

use super::{Declarator, ScopedName};
use crate::hir::{ConstrTypeDcl, Definition, Export, InterfaceDclInner, TypeDcl, TypedefType};
use std::collections::BTreeSet;

/// Names declared in one specification, resolved with IDL lexical scope.
#[derive(Default)]
pub(crate) struct DeclarationNames(BTreeSet<String>);

impl DeclarationNames {
    /// Collects all declarations, including names exported by interfaces.
    pub(crate) fn collect(definitions: &[Definition]) -> Self {
        let mut names = Self::default();
        Self::visit(definitions, "", &mut |name| {
            names.0.insert(name.to_string());
        });
        names
    }

    /// Resolves a known reference without rebinding it in a consumer's scope.
    pub(crate) fn resolve(&self, scope: &[String], name: &ScopedName) -> Option<ScopedName> {
        let depth = if name.is_root { 0 } else { scope.len() };
        for depth in (0..=depth).rev() {
            let mut candidate = scope[..depth].to_vec();
            candidate.extend(name.name.iter().cloned());
            if self.0.contains(&candidate.join("::")) {
                return Some(ScopedName {
                    is_root: true,
                    name: candidate,
                });
            }
        }
        None
    }

    /// Visits the path of every named declaration in source order.
    pub(crate) fn visit(definitions: &[Definition], path: &str, visit: &mut impl FnMut(&str)) {
        for definition in definitions {
            match definition {
                Definition::Pragma(_) => {}
                Definition::ModuleDcl(module) => {
                    let here = Self::join(path, &module.ident);
                    visit(&here);
                    Self::visit(&module.definition, &here, visit);
                }
                Definition::TypeDcl(type_dcl) => Self::type_dcl(type_dcl, path, visit),
                Definition::ConstrTypeDcl(constr) => Self::constr(constr, path, visit),
                Definition::ConstDcl(const_dcl) => visit(&Self::join(path, &const_dcl.ident)),
                Definition::ExceptDcl(except) => visit(&Self::join(path, &except.ident)),
                Definition::InterfaceDcl(interface) => {
                    let ident = match &interface.decl {
                        InterfaceDclInner::InterfaceForwardDcl(forward) => forward.ident.clone(),
                        InterfaceDclInner::InterfaceDef(def) => def.header.ident.clone(),
                    };
                    let here = Self::join(path, &ident);
                    visit(&here);
                    let InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
                        continue;
                    };
                    let Some(body) = &def.interface_body else {
                        continue;
                    };
                    for export in &body.0 {
                        match export {
                            Export::TypeDcl(type_dcl) => Self::type_dcl(type_dcl, &here, visit),
                            Export::ExceptDcl(except) => visit(&Self::join(&here, &except.ident)),
                            Export::ConstDcl(const_dcl) => {
                                visit(&Self::join(&here, &const_dcl.ident))
                            }
                            Export::OpDcl(_) | Export::AttrDcl(_) => {}
                        }
                    }
                }
            }
        }
    }

    fn type_dcl(type_dcl: &TypeDcl, path: &str, visit: &mut impl FnMut(&str)) {
        match type_dcl {
            TypeDcl::ConstrTypeDcl(constr) => Self::constr(constr, path, visit),
            TypeDcl::TypedefDcl(typedef) => {
                if let TypedefType::ConstrTypeDcl(constr) = &typedef.ty {
                    Self::constr(constr, path, visit);
                }
                for declarator in &typedef.decl {
                    visit(&Self::join(path, Self::declarator_name(declarator)));
                }
            }
            TypeDcl::NativeDcl(native) => visit(&Self::join(path, &native.decl.0)),
        }
    }

    fn constr(constr: &ConstrTypeDcl, path: &str, visit: &mut impl FnMut(&str)) {
        let (ident, members): (&str, Vec<&str>) = match constr {
            ConstrTypeDcl::StructDcl(value) => (&value.ident, Vec::new()),
            ConstrTypeDcl::StructForwardDcl(value) => (&value.ident, Vec::new()),
            ConstrTypeDcl::EnumDcl(value) => (
                &value.ident,
                value
                    .member
                    .iter()
                    .map(|member| member.ident.as_str())
                    .collect(),
            ),
            ConstrTypeDcl::UnionDef(value) => (&value.ident, Vec::new()),
            ConstrTypeDcl::UnionForwardDcl(value) => (&value.ident, Vec::new()),
            ConstrTypeDcl::BitsetDcl(value) => (&value.ident, Vec::new()),
            ConstrTypeDcl::BitmaskDcl(value) => (
                &value.ident,
                value.value.iter().map(|bit| bit.ident.as_str()).collect(),
            ),
        };
        let here = Self::join(path, ident);
        visit(&here);
        for member in members {
            visit(&Self::join(&here, member));
        }
    }

    fn join(path: &str, ident: &str) -> String {
        if path.is_empty() {
            ident.to_string()
        } else {
            format!("{path}::{ident}")
        }
    }

    fn declarator_name(declarator: &Declarator) -> &str {
        match declarator {
            Declarator::SimpleDeclarator(simple) => &simple.0,
            Declarator::ArrayDeclarator(array) => &array.ident,
        }
    }
}
