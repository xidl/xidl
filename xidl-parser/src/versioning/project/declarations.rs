//! Paths of named declarations, used to bookkeep references across a projection.

use super::super::paths::{declarator_name, join};
use crate::hir::{ConstrTypeDcl, Definition, Export, InterfaceDclInner, TypeDcl};

/// Visits the path of every named declaration in source order.
pub(super) fn for_each_declaration(
    definitions: &[Definition],
    path: &str,
    visit: &mut impl FnMut(&str),
) {
    for definition in definitions {
        match definition {
            Definition::Pragma(_) => {}
            Definition::ModuleDcl(module) => {
                let here = join(path, &module.ident);
                visit(&here);
                for_each_declaration(&module.definition, &here, visit);
            }
            Definition::TypeDcl(type_dcl) => for_each_type_dcl(type_dcl, path, visit),
            Definition::ConstrTypeDcl(constr) => for_each_constr(constr, path, visit),
            Definition::ConstDcl(const_dcl) => visit(&join(path, &const_dcl.ident)),
            Definition::ExceptDcl(except) => visit(&join(path, &except.ident)),
            Definition::InterfaceDcl(interface) => {
                let ident = match &interface.decl {
                    InterfaceDclInner::InterfaceForwardDcl(forward) => forward.ident.clone(),
                    InterfaceDclInner::InterfaceDef(def) => def.header.ident.clone(),
                };
                let here = join(path, &ident);
                visit(&here);
                let InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
                    continue;
                };
                let Some(body) = &def.interface_body else {
                    continue;
                };
                for export in &body.0 {
                    match export {
                        Export::TypeDcl(type_dcl) => for_each_type_dcl(type_dcl, &here, visit),
                        Export::ExceptDcl(except) => visit(&join(&here, &except.ident)),
                        Export::ConstDcl(const_dcl) => visit(&join(&here, &const_dcl.ident)),
                        Export::OpDcl(_) | Export::AttrDcl(_) => {}
                    }
                }
            }
        }
    }
}

fn for_each_type_dcl(type_dcl: &TypeDcl, path: &str, visit: &mut impl FnMut(&str)) {
    match type_dcl {
        TypeDcl::ConstrTypeDcl(constr) => for_each_constr(constr, path, visit),
        TypeDcl::TypedefDcl(typedef) => {
            for declarator in &typedef.decl {
                visit(&join(path, &declarator_name(declarator)));
            }
        }
        TypeDcl::NativeDcl(native) => visit(&join(path, &native.decl.0)),
    }
}

fn for_each_constr(constr: &ConstrTypeDcl, path: &str, visit: &mut impl FnMut(&str)) {
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
    let here = join(path, ident);
    visit(&here);
    for member in members {
        visit(&join(&here, member));
    }
}
