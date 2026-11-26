//! Resolves wire discriminator tags for union cases switching on renamed enums.

use crate::hir::{
    CaseLabel, ConstExpr, ConstrTypeDcl, Definition, ElementSpecTy, EnumDcl, Export,
    InterfaceDclInner, ScopedName, Specification, TypeDcl, TypeSpec, TypedefType, UnionDef,
    effective_wire_name,
};
use std::collections::{HashMap, HashSet};

/// Populates `Case::wire_tags` when an enum case label resolves to a renamed enumerator.
#[derive(Default)]
pub(crate) struct UnionWireTags {
    enums: HashMap<Vec<String>, HashMap<String, String>>,
    aliases: HashMap<Vec<String>, (Vec<String>, ScopedName)>,
}

impl UnionWireTags {
    /// Resolves renamed discriminator tags across every union in `spec`.
    pub(crate) fn resolve(spec: &mut Specification) {
        let mut resolver = Self::default();
        resolver.collect(&spec.0, &[]);
        resolver.apply(&mut spec.0, &[]);
    }

    fn collect(&mut self, definitions: &[Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                Definition::ModuleDcl(module) => {
                    self.collect(&module.definition, &Self::join(scope, &module.ident));
                }
                Definition::TypeDcl(ty) => self.collect_type(ty, scope),
                Definition::ConstrTypeDcl(ty) => self.collect_constructed(ty, scope),
                Definition::InterfaceDcl(interface) => {
                    if let InterfaceDclInner::InterfaceDef(definition) = &interface.decl
                        && let Some(body) = &definition.interface_body
                    {
                        let nested = Self::join(scope, &definition.header.ident);
                        for export in &body.0 {
                            if let Export::TypeDcl(ty) = export {
                                self.collect_type(ty, &nested);
                            }
                        }
                    }
                }
                Definition::ConstDcl(_) | Definition::ExceptDcl(_) | Definition::Pragma(_) => {}
            }
        }
    }

    fn collect_type(&mut self, ty: &TypeDcl, scope: &[String]) {
        match ty {
            TypeDcl::ConstrTypeDcl(constr) => self.collect_constructed(constr, scope),
            TypeDcl::TypedefDcl(typedef) => match &typedef.ty {
                TypedefType::ConstrTypeDcl(constr) => self.collect_constructed(constr, scope),
                TypedefType::TypeSpec(TypeSpec::ScopedName(target)) => {
                    for decl in &typedef.decl {
                        if let crate::hir::Declarator::SimpleDeclarator(value) = decl {
                            self.aliases.insert(
                                Self::join(scope, &value.0),
                                (scope.to_vec(), target.clone()),
                            );
                        }
                    }
                }
                TypedefType::TypeSpec(_) => {}
            },
            TypeDcl::NativeDcl(_) => {}
        }
    }

    fn collect_constructed(&mut self, ty: &ConstrTypeDcl, scope: &[String]) {
        match ty {
            ConstrTypeDcl::EnumDcl(enumeration) => self.record_enum(enumeration, scope),
            ConstrTypeDcl::UnionDef(union) => {
                let nested = Self::join(scope, &union.ident);
                for case in &union.case {
                    if let ElementSpecTy::ConstrTypeDcl(constr) = &case.element.ty {
                        self.collect_constructed(constr, &nested);
                    }
                }
            }
            _ => {}
        }
    }

    fn record_enum(&mut self, enumeration: &EnumDcl, scope: &[String]) {
        let members = enumeration
            .member
            .iter()
            .map(|member| {
                let wire = effective_wire_name(
                    &member.ident,
                    &member.annotations,
                    &enumeration.annotations,
                );
                (member.ident.clone(), wire)
            })
            .collect();
        self.enums
            .insert(Self::join(scope, &enumeration.ident), members);
    }

    fn apply(&self, definitions: &mut [Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                Definition::ModuleDcl(module) => {
                    let nested = Self::join(scope, &module.ident);
                    self.apply(&mut module.definition, &nested);
                }
                Definition::TypeDcl(ty) => self.apply_type(ty, scope),
                Definition::ConstrTypeDcl(ty) => self.apply_constructed(ty, scope),
                Definition::InterfaceDcl(interface) => {
                    if let InterfaceDclInner::InterfaceDef(definition) = &mut interface.decl
                        && let Some(body) = &mut definition.interface_body
                    {
                        let nested = Self::join(scope, &definition.header.ident);
                        for export in &mut body.0 {
                            if let Export::TypeDcl(ty) = export {
                                self.apply_type(ty, &nested);
                            }
                        }
                    }
                }
                Definition::ConstDcl(_) | Definition::ExceptDcl(_) | Definition::Pragma(_) => {}
            }
        }
    }

    fn apply_type(&self, ty: &mut TypeDcl, scope: &[String]) {
        match ty {
            TypeDcl::ConstrTypeDcl(constr) => self.apply_constructed(constr, scope),
            TypeDcl::TypedefDcl(typedef) => {
                if let TypedefType::ConstrTypeDcl(constr) = &mut typedef.ty {
                    self.apply_constructed(constr, scope);
                }
            }
            TypeDcl::NativeDcl(_) => {}
        }
    }

    fn apply_constructed(&self, ty: &mut ConstrTypeDcl, scope: &[String]) {
        if let ConstrTypeDcl::UnionDef(union) = ty {
            self.apply_union(union, scope);
        }
    }

    fn apply_union(&self, union: &mut UnionDef, scope: &[String]) {
        let nested = Self::join(scope, &union.ident);
        for case in &mut union.case {
            if let ElementSpecTy::ConstrTypeDcl(constr) = &mut case.element.ty {
                self.apply_constructed(constr, &nested);
            }
            let resolved: Vec<Option<String>> = case
                .label
                .iter()
                .map(|label| self.resolve_label(label, scope))
                .collect();
            if resolved.iter().any(Option::is_some) {
                case.wire_tags = resolved;
            } else {
                case.wire_tags.clear();
            }
        }
    }

    fn resolve_label(&self, label: &CaseLabel, scope: &[String]) -> Option<String> {
        let CaseLabel::Value(ConstExpr::ScopedName(name)) = label else {
            return None;
        };
        let (member, owner) = name.name.split_last()?;
        if owner.is_empty() {
            return None;
        }
        let enum_name = ScopedName {
            is_root: name.is_root,
            name: owner.to_vec(),
        };
        let members = self.lookup_enum(&enum_name, scope, &mut HashSet::new())?;
        let wire = members.get(member)?;
        (wire != member).then(|| wire.clone())
    }

    fn lookup_enum<'a>(
        &'a self,
        name: &ScopedName,
        scope: &[String],
        visited: &mut HashSet<Vec<String>>,
    ) -> Option<&'a HashMap<String, String>> {
        let candidates: Vec<Vec<String>> = if name.is_root {
            vec![name.name.clone()]
        } else {
            (0..=scope.len())
                .rev()
                .map(|depth| {
                    let mut path = scope[..depth].to_vec();
                    path.extend_from_slice(&name.name);
                    path
                })
                .collect()
        };
        for path in candidates {
            if let Some(members) = self.enums.get(&path) {
                return Some(members);
            }
            if let Some((alias_scope, target)) = self.aliases.get(&path) {
                if !visited.insert(path) {
                    return None;
                }
                return self.lookup_enum(target, alias_scope, visited);
            }
        }
        None
    }

    fn join(scope: &[String], ident: &str) -> Vec<String> {
        let mut path = scope.to_vec();
        path.push(ident.to_string());
        path
    }
}
