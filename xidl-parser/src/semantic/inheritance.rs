//! Single-base struct inheritance and its flattened wire shape.

use crate::error::{ParseError, ParserResult};
use crate::hir::{
    Annotation, ConstrTypeDcl, Declarator, Definition, Member, ScopedName, Specification,
    StructDcl, TypeDcl,
};
use crate::hir::{effective_wire_name, field_rename};
use std::collections::{BTreeMap, BTreeSet};

/// Validates struct inheritance and records inherited members on each struct.
///
/// A struct has at most one base struct, the base must name a complete struct,
/// the chain must not cycle, and no field may repeat a field declared by an
/// ancestor. Inherited members keep their wire names so a target can flatten
/// them without re-deriving the base struct's annotations.
pub(crate) fn analyze(spec: &mut Specification) -> ParserResult<()> {
    let structs = collect_structs(&spec.0);
    let inherited = flatten(&structs)?;
    apply(&mut spec.0, &[], &inherited);
    Ok(())
}

fn collect_structs(defs: &[Definition]) -> BTreeMap<String, StructDcl> {
    let mut out = BTreeMap::new();
    collect_defs(defs, &[], &mut out);
    out
}

fn collect_defs(defs: &[Definition], scope: &[String], out: &mut BTreeMap<String, StructDcl>) {
    for def in defs {
        match def {
            Definition::ModuleDcl(module) => {
                let mut nested = scope.to_vec();
                nested.push(module.ident.clone());
                collect_defs(&module.definition, &nested, out);
            }
            Definition::ConstrTypeDcl(constr) => collect_constr(constr, scope, out),
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                collect_constr(constr, scope, out)
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(alias)) => {
                if let crate::hir::TypedefType::ConstrTypeDcl(constr) = &alias.ty {
                    collect_constr(constr, scope, out);
                }
            }
            Definition::InterfaceDcl(interface) => {
                let crate::hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
                    continue;
                };
                let mut nested = scope.to_vec();
                nested.push(def.header.ident.clone());
                let Some(body) = &def.interface_body else {
                    continue;
                };
                for export in &body.0 {
                    if let crate::hir::Export::TypeDcl(type_dcl) = export {
                        match type_dcl {
                            TypeDcl::ConstrTypeDcl(constr) => collect_constr(constr, &nested, out),
                            TypeDcl::TypedefDcl(alias) => {
                                if let crate::hir::TypedefType::ConstrTypeDcl(constr) = &alias.ty {
                                    collect_constr(constr, &nested, out);
                                }
                            }
                            TypeDcl::NativeDcl(_) => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_constr(constr: &ConstrTypeDcl, scope: &[String], out: &mut BTreeMap<String, StructDcl>) {
    if let ConstrTypeDcl::StructDcl(def) = constr {
        out.insert(canonical(scope, &def.ident), def.clone());
    }
}

/// Returns the inherited members of every struct, ancestor first.
fn flatten(structs: &BTreeMap<String, StructDcl>) -> ParserResult<BTreeMap<String, Vec<Member>>> {
    let mut out = BTreeMap::new();
    for key in structs.keys() {
        let mut chain = Vec::new();
        let mut cursor = Some(key.clone());
        while let Some(current) = cursor {
            if chain.iter().any(|seen| seen == &current) {
                return Err(error(format!(
                    "struct inheritance cycle includes '{current}' (from '{key}')"
                )));
            }
            let def = structs.get(&current).ok_or_else(|| {
                error(format!(
                    "struct '{key}' inherits '{current}', which is not a complete struct"
                ))
            })?;
            cursor = parent_of(&current, def, structs)?;
            chain.push(current);
        }
        let mut inherited = Vec::new();
        for ancestor in chain.iter().skip(1).rev() {
            let def = &structs[ancestor];
            for member in &def.member {
                inherited.extend(wire_named_members(member, &def.annotations));
            }
        }
        validate_duplicates(key, &structs[key], &inherited)?;
        out.insert(key.clone(), inherited);
    }
    Ok(out)
}

fn parent_of(
    key: &str,
    def: &StructDcl,
    structs: &BTreeMap<String, StructDcl>,
) -> ParserResult<Option<String>> {
    let Some(parent) = def.parent.first() else {
        return Ok(None);
    };
    if def.parent.len() > 1 {
        return Err(error(format!(
            "struct '{}' inherits {} structs; XIDL supports one base struct",
            key,
            def.parent.len()
        )));
    }
    let scope = scope_of(key);
    resolve(&scope, parent, structs).map(Some).ok_or_else(|| {
        error(format!(
            "struct '{key}' inherits '{}', which is not a complete struct",
            parent.name.join("::")
        ))
    })
}

fn validate_duplicates(key: &str, def: &StructDcl, inherited: &[Member]) -> ParserResult<()> {
    let mut inherited_names = BTreeSet::new();
    for member in inherited {
        if crate::hir::is_skipped(&member.annotations) {
            continue;
        }
        for decl in &member.ident {
            inherited_names.insert(declarator_name(decl).to_string());
        }
    }
    for member in &def.member {
        if crate::hir::is_skipped(&member.annotations) {
            continue;
        }
        for decl in &member.ident {
            let name = declarator_name(decl);
            if inherited_names.contains(name) {
                return Err(error(format!(
                    "struct '{key}' field '{name}' duplicates a field declared by an ancestor"
                )));
            }
        }
    }
    Ok(())
}

/// Copies a base member with its effective wire name pinned as `@rename`.
fn wire_named_members(member: &Member, container: &[Annotation]) -> Vec<Member> {
    member
        .ident
        .iter()
        .map(|decl| {
            let raw = declarator_name(decl);
            let wire = effective_wire_name(raw, &member.annotations, container);
            let mut annotations = member.annotations.clone();
            if field_rename(&annotations).is_none() {
                annotations.push(Annotation::Rename { name: wire });
            }
            Member {
                annotations,
                ty: member.ty.clone(),
                ident: vec![decl.clone()],
                default: member.default.clone(),
                field_id: member.field_id,
                recursive: member.recursive,
                references_deprecated: member.references_deprecated,
            }
        })
        .collect()
}

fn apply(defs: &mut [Definition], scope: &[String], inherited: &BTreeMap<String, Vec<Member>>) {
    for def in defs {
        match def {
            Definition::ModuleDcl(module) => {
                let mut nested = scope.to_vec();
                nested.push(module.ident.clone());
                apply(&mut module.definition, &nested, inherited);
            }
            Definition::ConstrTypeDcl(constr) => apply_constr(constr, scope, inherited),
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                apply_constr(constr, scope, inherited)
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(alias)) => {
                if let crate::hir::TypedefType::ConstrTypeDcl(constr) = &mut alias.ty {
                    apply_constr(constr, scope, inherited);
                }
            }
            Definition::InterfaceDcl(interface) => {
                let crate::hir::InterfaceDclInner::InterfaceDef(def) = &mut interface.decl else {
                    continue;
                };
                let mut nested = scope.to_vec();
                nested.push(def.header.ident.clone());
                let Some(body) = &mut def.interface_body else {
                    continue;
                };
                for export in &mut body.0 {
                    if let crate::hir::Export::TypeDcl(type_dcl) = export {
                        match type_dcl {
                            TypeDcl::ConstrTypeDcl(constr) => {
                                apply_constr(constr, &nested, inherited)
                            }
                            TypeDcl::TypedefDcl(alias) => {
                                if let crate::hir::TypedefType::ConstrTypeDcl(constr) =
                                    &mut alias.ty
                                {
                                    apply_constr(constr, &nested, inherited);
                                }
                            }
                            TypeDcl::NativeDcl(_) => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn apply_constr(
    constr: &mut ConstrTypeDcl,
    scope: &[String],
    inherited: &BTreeMap<String, Vec<Member>>,
) {
    if let ConstrTypeDcl::StructDcl(def) = constr {
        def.inherited = inherited
            .get(&canonical(scope, &def.ident))
            .cloned()
            .unwrap_or_default();
    }
}

fn resolve(
    scope: &[String],
    name: &ScopedName,
    structs: &BTreeMap<String, StructDcl>,
) -> Option<String> {
    if name.is_root {
        let key = name.name.join("::");
        return structs.contains_key(&key).then_some(key);
    }
    for depth in (0..=scope.len()).rev() {
        let mut candidate = scope[..depth].to_vec();
        candidate.extend(name.name.iter().cloned());
        let key = candidate.join("::");
        if structs.contains_key(&key) {
            return Some(key);
        }
    }
    None
}

fn canonical(scope: &[String], ident: &str) -> String {
    scope
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(ident))
        .collect::<Vec<_>>()
        .join("::")
}

fn scope_of(canonical: &str) -> Vec<String> {
    let mut parts = canonical
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    parts.pop();
    parts
}

fn declarator_name(decl: &Declarator) -> &str {
    match decl {
        Declarator::SimpleDeclarator(value) => &value.0,
        Declarator::ArrayDeclarator(value) => &value.ident,
    }
}

fn error(message: String) -> ParseError {
    ParseError::Message(format!("struct inheritance: {message}"))
}
