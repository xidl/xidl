//! Module-level `@deprecated` inheritance for the declarations inside it.

use crate::hir::{
    Annotation, ConstrTypeDcl, Declarator, Definition, ElementSpecTy, ScopedName, Specification,
    TypeDcl, TypeSpec, TypedefType, annotation_name,
};
use std::collections::{BTreeMap, HashSet};

impl Specification {
    /// Appends the innermost enclosing module's `@deprecated` to every
    /// declaration that lacks its own, so a target inherits module
    /// deprecation without re-walking module scopes. The most specific
    /// annotation already on a declaration wins.
    pub(crate) fn inherit_module_deprecation(&mut self) {
        // A module may arrive split across several blocks; targets merge
        // them into one declaration, so a block's annotation governs every
        // block at the same module path.
        let mut deprecated_modules = BTreeMap::new();
        collect_module_deprecation(&self.0, &[], &mut deprecated_modules);
        ModuleDeprecation {
            deprecated_modules: &deprecated_modules,
        }
        .apply(&mut self.0, &[], None);
    }

    /// Flags members and aliases whose type names a `@deprecated`
    /// declaration, so generated mechanical references can silence rustc's
    /// `deprecated` lint without weakening it for contract consumers.
    pub(crate) fn mark_deprecated_references(&mut self) {
        let mut deprecated = HashSet::new();
        collect_deprecated(&self.0, &[], &mut deprecated);
        mark_references(&mut self.0, &[], &deprecated);
    }
}

/// The `@deprecated` of each deprecated module path; the first block wins
/// when a path carries more than one.
type ModuleDeprecations = BTreeMap<Vec<String>, Annotation>;

fn collect_module_deprecation(
    definitions: &[Definition],
    path: &[String],
    out: &mut ModuleDeprecations,
) {
    for definition in definitions {
        if let Definition::ModuleDcl(module) = definition {
            let mut nested = path.to_vec();
            nested.push(module.ident.clone());
            if let Some(annotation) = deprecated(&module.annotations) {
                out.entry(nested.clone())
                    .or_insert_with(|| annotation.clone());
            }
            collect_module_deprecation(&module.definition, &nested, out);
        }
    }
}

struct ModuleDeprecation<'a> {
    deprecated_modules: &'a ModuleDeprecations,
}

impl ModuleDeprecation<'_> {
    /// `governing` is the deprecation already ruling the declarations of
    /// this level: the enclosing module's own annotation, or its nearest
    /// deprecated ancestor's.
    fn apply(
        &self,
        definitions: &mut [Definition],
        path: &[String],
        governing: Option<&Annotation>,
    ) {
        for definition in definitions {
            match definition {
                Definition::ModuleDcl(module) => {
                    let mut nested = path.to_vec();
                    nested.push(module.ident.clone());
                    let own = self.deprecated_modules.get(&nested).or(governing);
                    self.apply(&mut module.definition, &nested, own);
                }
                other => other.inherit_deprecation(governing),
            }
        }
    }
}

impl Definition {
    fn inherit_deprecation(&mut self, inherited: Option<&Annotation>) {
        let Some(inherited) = inherited else {
            return;
        };
        match self {
            Definition::ConstrTypeDcl(constr) => constr.inherit_deprecation(inherited),
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                constr.inherit_deprecation(inherited)
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(alias)) => {
                push_inherited(&mut alias.annotations, inherited)
            }
            Definition::ExceptDcl(except) => push_inherited(&mut except.annotations, inherited),
            Definition::InterfaceDcl(interface) => {
                push_inherited(&mut interface.annotations, inherited)
            }
            _ => {}
        }
    }
}

impl ConstrTypeDcl {
    fn inherit_deprecation(&mut self, inherited: &Annotation) {
        let annotations = match self {
            ConstrTypeDcl::StructDcl(def) => &mut def.annotations,
            ConstrTypeDcl::EnumDcl(def) => &mut def.annotations,
            ConstrTypeDcl::UnionDef(def) => &mut def.annotations,
            ConstrTypeDcl::BitsetDcl(def) => &mut def.annotations,
            ConstrTypeDcl::BitmaskDcl(def) => &mut def.annotations,
            _ => return,
        };
        push_inherited(annotations, inherited);
    }
}

/// The first `@deprecated` annotation in the list, if any.
fn deprecated(annotations: &[Annotation]) -> Option<&Annotation> {
    annotations
        .iter()
        .find(|annotation| is_deprecated(annotation))
}

fn is_deprecated(annotation: &Annotation) -> bool {
    annotation_name(annotation)
        .map(|name| name.eq_ignore_ascii_case("deprecated"))
        .unwrap_or(false)
}

fn push_inherited(annotations: &mut Vec<Annotation>, inherited: &Annotation) {
    if deprecated(annotations).is_none() {
        annotations.push(inherited.clone());
    }
}

fn collect_deprecated(
    definitions: &[Definition],
    scope: &[String],
    out: &mut HashSet<Vec<String>>,
) {
    for definition in definitions {
        match definition {
            Definition::ModuleDcl(module) => {
                let mut nested = scope.to_vec();
                nested.push(module.ident.clone());
                collect_deprecated(&module.definition, &nested, out);
            }
            Definition::ConstrTypeDcl(constr) => {
                collect_constr(constr, scope, out);
            }
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                collect_constr(constr, scope, out);
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(alias)) => {
                if deprecated(&alias.annotations).is_some() {
                    for decl in &alias.decl {
                        let mut path = scope.to_vec();
                        path.push(alias_name(decl).to_string());
                        out.insert(path);
                    }
                }
            }
            Definition::ExceptDcl(except) => {
                if deprecated(&except.annotations).is_some() {
                    let mut path = scope.to_vec();
                    path.push(except.ident.clone());
                    out.insert(path);
                }
            }
            _ => {}
        }
    }
}

fn collect_constr(constr: &ConstrTypeDcl, scope: &[String], out: &mut HashSet<Vec<String>>) {
    if deprecated(constr.annotations()).is_some() {
        let mut path = scope.to_vec();
        path.push(constr.ident().to_string());
        out.insert(path);
    }
}

fn mark_references(
    definitions: &mut [Definition],
    scope: &[String],
    deprecated: &HashSet<Vec<String>>,
) {
    for definition in definitions {
        match definition {
            Definition::ModuleDcl(module) => {
                let mut nested = scope.to_vec();
                nested.push(module.ident.clone());
                mark_references(&mut module.definition, &nested, deprecated);
            }
            Definition::ConstrTypeDcl(constr) => {
                constr.mark_references(scope, deprecated);
            }
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                constr.mark_references(scope, deprecated);
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(alias)) => {
                let references = match &alias.ty {
                    TypedefType::TypeSpec(ty) => references_deprecated(scope, ty, deprecated),
                    TypedefType::ConstrTypeDcl(_) => false,
                };
                alias.references_deprecated = references;
            }
            Definition::ExceptDcl(except) => {
                for member in &mut except.member {
                    member.references_deprecated =
                        references_deprecated(scope, &member.ty, deprecated);
                }
            }
            _ => {}
        }
    }
}

impl ConstrTypeDcl {
    fn mark_references(&mut self, scope: &[String], deprecated: &HashSet<Vec<String>>) {
        match self {
            ConstrTypeDcl::StructDcl(def) => {
                for member in def.inherited.iter_mut().chain(def.member.iter_mut()) {
                    member.references_deprecated =
                        references_deprecated(scope, &member.ty, deprecated);
                }
            }
            ConstrTypeDcl::UnionDef(def) => {
                for case in &mut def.case {
                    if let ElementSpecTy::TypeSpec(ty) = &case.element.ty {
                        case.element.references_deprecated =
                            references_deprecated(scope, ty, deprecated);
                    }
                }
            }
            _ => {}
        }
    }

    fn annotations(&self) -> &[Annotation] {
        match self {
            ConstrTypeDcl::StructDcl(def) => &def.annotations,
            ConstrTypeDcl::EnumDcl(def) => &def.annotations,
            ConstrTypeDcl::UnionDef(def) => &def.annotations,
            ConstrTypeDcl::BitsetDcl(def) => &def.annotations,
            ConstrTypeDcl::BitmaskDcl(def) => &def.annotations,
            ConstrTypeDcl::StructForwardDcl(def) => &def.annotations,
            ConstrTypeDcl::UnionForwardDcl(def) => &def.annotations,
        }
    }
}

/// Whether any scoped name inside the type resolves to a deprecated
/// declaration, resolving relative names against the enclosing scopes.
fn references_deprecated(
    scope: &[String],
    ty: &TypeSpec,
    deprecated: &HashSet<Vec<String>>,
) -> bool {
    match ty {
        TypeSpec::ScopedName(name) => resolve(scope, name, deprecated),
        TypeSpec::SequenceType(sequence) => references_deprecated(scope, &sequence.ty, deprecated),
        TypeSpec::MapType(map) => {
            references_deprecated(scope, &map.key, deprecated)
                || references_deprecated(scope, &map.value, deprecated)
        }
        TypeSpec::TemplateType(template) => template
            .args
            .iter()
            .any(|arg| references_deprecated(scope, arg, deprecated)),
        _ => false,
    }
}

fn alias_name(value: &Declarator) -> &str {
    match value {
        Declarator::SimpleDeclarator(value) => &value.0,
        Declarator::ArrayDeclarator(value) => &value.ident,
    }
}

fn resolve(scope: &[String], name: &ScopedName, deprecated: &HashSet<Vec<String>>) -> bool {
    if name.is_root {
        return deprecated.contains(&name.name);
    }
    for depth in (0..=scope.len()).rev() {
        let mut candidate = scope[..depth].to_vec();
        candidate.extend(name.name.iter().cloned());
        if deprecated.contains(&candidate) {
            return true;
        }
    }
    false
}
