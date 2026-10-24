//! Resolves union switch types to their declaring enums.
//!
//! A union case label may name an enumerator of the switch enum or a
//! constant of that enum type, and the HIR keeps either exactly as written.
//! Generators need to tell the two apart — a bare `sms` must become a
//! qualified variant path while a constant renders as is — so this pass
//! records the switch enum's declaration path and enumerators on every union
//! whose switch type is, or aliases, a declared enum.

use crate::hir::{
    ConstrTypeDcl, Declarator, Definition, EnumDcl, ScopedName, SimpleDeclarator, Specification,
    SwitchEnum, SwitchTypeSpec, TypeDcl, TypeSpec, TypedefType,
};
use crate::semantic::recursive::resolve_in;
use std::collections::{HashMap, HashSet};

/// Where a typedef alias was declared and what it names.
struct AliasTarget {
    target: ScopedName,
    scope: Vec<String>,
}

/// Declared enums by path, and typedef aliases by path.
struct Declared {
    enums: HashMap<Vec<String>, Vec<String>>,
    aliases: HashMap<Vec<String>, AliasTarget>,
}

/// Records each union's resolved switch enum on the union. Unresolvable
/// switch types stay `None`, and generators keep rendering their labels as
/// written.
pub(crate) fn annotate_switch_enums(spec: &mut Specification) {
    let mut declared = Declared {
        enums: HashMap::new(),
        aliases: HashMap::new(),
    };
    collect(&spec.0, &[], &mut declared);
    annotate(&mut spec.0, &[], &declared);
}

fn collect(definitions: &[Definition], module_path: &[String], declared: &mut Declared) {
    for definition in definitions {
        match definition {
            Definition::ModuleDcl(module) => {
                let mut nested = module_path.to_vec();
                nested.push(module.ident.clone());
                collect(&module.definition, &nested, declared);
            }
            Definition::ConstrTypeDcl(ConstrTypeDcl::EnumDcl(def)) => {
                declared.insert_enum(module_path, def);
            }
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(ConstrTypeDcl::EnumDcl(def))) => {
                declared.insert_enum(module_path, def);
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(typedef)) => match &typedef.ty {
                TypedefType::ConstrTypeDcl(ConstrTypeDcl::EnumDcl(def)) => {
                    declared.insert_enum(module_path, def);
                }
                TypedefType::TypeSpec(TypeSpec::ScopedName(target)) => {
                    for declarator in &typedef.decl {
                        if let Declarator::SimpleDeclarator(SimpleDeclarator(ident)) = declarator {
                            declared.insert_alias(module_path, ident, target);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

impl Declared {
    fn insert_enum(&mut self, module_path: &[String], def: &EnumDcl) {
        let enumerators = def
            .member
            .iter()
            .map(|enumerator| enumerator.ident.clone())
            .collect();
        self.enums
            .insert(declaration_path(module_path, &def.ident), enumerators);
    }

    fn insert_alias(&mut self, module_path: &[String], ident: &str, target: &ScopedName) {
        self.aliases.insert(
            declaration_path(module_path, ident),
            AliasTarget {
                target: target.clone(),
                scope: module_path.to_vec(),
            },
        );
    }

    /// Resolves a union's switch type to its enum, following typedef aliases
    /// (each resolved in the scope that declared it) and refusing alias
    /// cycles.
    fn switch_enum(
        &self,
        switch_type_spec: &SwitchTypeSpec,
        scope: &[String],
    ) -> Option<SwitchEnum> {
        let SwitchTypeSpec::ScopedName(name) = switch_type_spec else {
            return None;
        };
        let mut scope = scope.to_vec();
        let mut current = name.clone();
        let mut visited = HashSet::new();
        loop {
            if !visited.insert((current.is_root, current.name.clone())) {
                return None;
            }
            if let Some(path) = resolve_in(&scope, &current, |path| self.enums.contains_key(path)) {
                let enumerators = self.enums[&path].clone();
                return Some(SwitchEnum { path, enumerators });
            }
            let key = resolve_in(&scope, &current, |path| self.aliases.contains_key(path))?;
            let AliasTarget {
                target,
                scope: declared,
            } = &self.aliases[&key];
            current = target.clone();
            scope = declared.clone();
        }
    }
}

fn annotate(definitions: &mut [Definition], module_path: &[String], declared: &Declared) {
    for definition in definitions {
        match definition {
            Definition::ModuleDcl(module) => {
                let mut nested = module_path.to_vec();
                nested.push(module.ident.clone());
                annotate(&mut module.definition, &nested, declared);
            }
            Definition::ConstrTypeDcl(ConstrTypeDcl::UnionDef(def))
            | Definition::TypeDcl(TypeDcl::ConstrTypeDcl(ConstrTypeDcl::UnionDef(def))) => {
                def.switch_enum = declared.switch_enum(&def.switch_type_spec, module_path);
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(typedef)) => {
                if let TypedefType::ConstrTypeDcl(ConstrTypeDcl::UnionDef(def)) = &mut typedef.ty {
                    def.switch_enum = declared.switch_enum(&def.switch_type_spec, module_path);
                }
            }
            _ => {}
        }
    }
}

fn declaration_path(module_path: &[String], ident: &str) -> Vec<String> {
    let mut path = module_path.to_vec();
    path.push(ident.to_string());
    path
}
