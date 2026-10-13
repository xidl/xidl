//! Structural walk over every annotation-carrying site of a specification.

use super::super::paths::{attribute_name, declarator_name, declarators_name, join};
use super::availability::{Availability, Site, SiteKind, VersionVisitor};
use crate::error::ParserResult;
use crate::hir::{
    Annotation, ConstrTypeDcl, Definition, Enumerator, Export, InterfaceDclInner, TypeDcl,
};

/// Walks every annotation-carrying site of a specification in source order.
pub(crate) fn walk<V: VersionVisitor>(
    definitions: &[Definition],
    path: &str,
    parent: Availability,
    visitor: &mut V,
) -> ParserResult<()> {
    for definition in definitions {
        walk_definition(definition, path, parent, visitor)?;
    }
    Ok(())
}

fn walk_definition<V: VersionVisitor>(
    definition: &Definition,
    path: &str,
    parent: Availability,
    visitor: &mut V,
) -> ParserResult<()> {
    match definition {
        Definition::Pragma(_) => Ok(()),
        Definition::ModuleDcl(module) => {
            let here = join(path, &module.ident);
            let inner = enter(
                visitor,
                SiteKind::Module,
                &here,
                path,
                &module.annotations,
                None,
                parent,
            )?;
            walk(&module.definition, &here, inner, visitor)
        }
        Definition::TypeDcl(type_dcl) => walk_type_dcl(type_dcl, path, parent, visitor),
        Definition::ConstrTypeDcl(constr) => walk_constr(constr, path, parent, visitor),
        Definition::ConstDcl(const_dcl) => {
            enter(
                visitor,
                SiteKind::Unsupported,
                &join(path, &const_dcl.ident),
                path,
                &const_dcl.annotations,
                None,
                parent,
            )?;
            Ok(())
        }
        Definition::ExceptDcl(except) => {
            let here = join(path, &except.ident);
            let inner = enter(
                visitor,
                SiteKind::Type,
                &here,
                path,
                &except.annotations,
                None,
                parent,
            )?;
            walk_members(visitor, &except.member, &here, inner)
        }
        Definition::InterfaceDcl(interface) => {
            let ident = match &interface.decl {
                InterfaceDclInner::InterfaceForwardDcl(forward) => forward.ident.clone(),
                InterfaceDclInner::InterfaceDef(def) => def.header.ident.clone(),
            };
            let here = join(path, &ident);
            let inner = enter(
                visitor,
                SiteKind::Interface,
                &here,
                path,
                &interface.annotations,
                None,
                parent,
            )?;
            let InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
                return Ok(());
            };
            let Some(body) = &def.interface_body else {
                return Ok(());
            };
            for export in &body.0 {
                walk_export(export, &here, inner, visitor)?;
            }
            Ok(())
        }
    }
}

fn walk_members<V: VersionVisitor>(
    visitor: &mut V,
    members: &[crate::hir::Member],
    path: &str,
    parent: Availability,
) -> ParserResult<()> {
    for member in members {
        enter(
            visitor,
            SiteKind::Member,
            &join(path, &declarators_name(&member.ident)),
            path,
            &member.annotations,
            None,
            parent,
        )?;
    }
    Ok(())
}

fn walk_export<V: VersionVisitor>(
    export: &Export,
    path: &str,
    parent: Availability,
    visitor: &mut V,
) -> ParserResult<()> {
    match export {
        Export::OpDcl(operation) => {
            let here = join(path, &operation.ident);
            let inner = enter(
                visitor,
                SiteKind::Operation,
                &here,
                path,
                &operation.annotations,
                None,
                parent,
            )?;
            if let Some(parameters) = &operation.parameter {
                for parameter in &parameters.0 {
                    enter(
                        visitor,
                        SiteKind::Parameter,
                        &join(&here, &parameter.declarator.0),
                        &here,
                        &parameter.annotations,
                        None,
                        inner,
                    )?;
                }
            }
            Ok(())
        }
        Export::AttrDcl(attribute) => {
            enter(
                visitor,
                SiteKind::Attribute,
                &join(path, &attribute_name(attribute)),
                path,
                &attribute.annotations,
                None,
                parent,
            )?;
            Ok(())
        }
        Export::TypeDcl(type_dcl) => walk_type_dcl(type_dcl, path, parent, visitor),
        Export::ConstDcl(const_dcl) => {
            enter(
                visitor,
                SiteKind::Unsupported,
                &join(path, &const_dcl.ident),
                path,
                &const_dcl.annotations,
                None,
                parent,
            )?;
            Ok(())
        }
        Export::ExceptDcl(except) => {
            let here = join(path, &except.ident);
            let inner = enter(
                visitor,
                SiteKind::Type,
                &here,
                path,
                &except.annotations,
                None,
                parent,
            )?;
            walk_members(visitor, &except.member, &here, inner)
        }
    }
}

fn walk_type_dcl<V: VersionVisitor>(
    type_dcl: &TypeDcl,
    path: &str,
    parent: Availability,
    visitor: &mut V,
) -> ParserResult<()> {
    match type_dcl {
        TypeDcl::ConstrTypeDcl(constr) => walk_constr(constr, path, parent, visitor),
        TypeDcl::TypedefDcl(typedef) => {
            enter(
                visitor,
                SiteKind::Type,
                &join(path, &declarators_name(&typedef.decl)),
                path,
                &typedef.annotations,
                None,
                parent,
            )?;
            Ok(())
        }
        TypeDcl::NativeDcl(native) => {
            enter(
                visitor,
                SiteKind::Type,
                &join(path, &native.decl.0),
                path,
                &native.annotations,
                None,
                parent,
            )?;
            Ok(())
        }
    }
}

fn walk_constr<V: VersionVisitor>(
    constr: &ConstrTypeDcl,
    path: &str,
    parent: Availability,
    visitor: &mut V,
) -> ParserResult<()> {
    match constr {
        ConstrTypeDcl::StructDcl(value) => {
            let here = join(path, &value.ident);
            let inner = enter(
                visitor,
                SiteKind::Type,
                &here,
                path,
                &value.annotations,
                None,
                parent,
            )?;
            walk_members(visitor, &value.member, &here, inner)
        }
        ConstrTypeDcl::StructForwardDcl(value) => {
            enter(
                visitor,
                SiteKind::Type,
                &join(path, &value.ident),
                path,
                &value.annotations,
                None,
                parent,
            )?;
            Ok(())
        }
        ConstrTypeDcl::EnumDcl(value) => {
            let here = join(path, &value.ident);
            let inner = enter(
                visitor,
                SiteKind::Type,
                &here,
                path,
                &value.annotations,
                Some(&value.member),
                parent,
            )?;
            for member in &value.member {
                enter(
                    visitor,
                    SiteKind::Enumerator,
                    &join(&here, &member.ident),
                    &here,
                    &member.annotations,
                    None,
                    inner,
                )?;
            }
            Ok(())
        }
        ConstrTypeDcl::UnionDef(value) => {
            let here = join(path, &value.ident);
            let inner = enter(
                visitor,
                SiteKind::Type,
                &here,
                path,
                &value.annotations,
                None,
                parent,
            )?;
            for case in &value.case {
                enter(
                    visitor,
                    SiteKind::Member,
                    &join(&here, &declarator_name(&case.element.value)),
                    &here,
                    &case.element.annotations,
                    None,
                    inner,
                )?;
            }
            Ok(())
        }
        ConstrTypeDcl::UnionForwardDcl(value) => {
            enter(
                visitor,
                SiteKind::Type,
                &join(path, &value.ident),
                path,
                &value.annotations,
                None,
                parent,
            )?;
            Ok(())
        }
        ConstrTypeDcl::BitsetDcl(value) => {
            enter(
                visitor,
                SiteKind::Type,
                &join(path, &value.ident),
                path,
                &value.annotations,
                None,
                parent,
            )?;
            Ok(())
        }
        ConstrTypeDcl::BitmaskDcl(value) => {
            let here = join(path, &value.ident);
            let inner = enter(
                visitor,
                SiteKind::Type,
                &here,
                path,
                &value.annotations,
                None,
                parent,
            )?;
            for bit in &value.value {
                enter(
                    visitor,
                    SiteKind::BitmaskValue,
                    &join(&here, &bit.ident),
                    &here,
                    &bit.annotations,
                    None,
                    inner,
                )?;
            }
            Ok(())
        }
    }
}

fn enter<V: VersionVisitor>(
    visitor: &mut V,
    kind: SiteKind,
    path: &str,
    parent_path: &str,
    annotations: &[Annotation],
    enum_members: Option<&[Enumerator]>,
    parent: Availability,
) -> ParserResult<Availability> {
    let site = Site {
        path: path.to_string(),
        parent_path: parent_path.to_string(),
        kind,
        annotations,
        enum_members,
    };
    visitor.enter(&site, parent)
}
