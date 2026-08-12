use std::collections::HashSet;

use crate::hir::{
    ConstrTypeDcl, Declarator, Definition, ElementSpecTy, Specification, TypeDcl, TypeSpec,
    TypedefType,
};
use crate::semantic::recursive::paths::{join_path, resolve_struct_path, struct_path};
use crate::semantic::recursive_graph::collect_recursive_edges;

/// Returns the canonical paths of schema types that participate in a
/// recursive cycle, including indirect cycles through sequence, map, and
/// template arguments.
pub fn recursive_schema_types(spec: &Specification) -> HashSet<Vec<String>> {
    let mut names = HashSet::new();
    collect_schema_names(&spec.0, &[], &mut names);
    let mut edges = Vec::new();
    collect_schema_edges(&spec.0, &[], &names, &mut edges);
    let recursive_edges = collect_recursive_edges(&edges);
    let mut recursive = HashSet::new();
    for (from, to) in recursive_edges {
        recursive.insert(split_path(&from));
        recursive.insert(split_path(&to));
    }
    recursive
}

fn collect_schema_names(
    defs: &[Definition],
    module_path: &[String],
    names: &mut HashSet<Vec<String>>,
) {
    for def in defs {
        match def {
            Definition::ModuleDcl(module) => {
                let mut nested = module_path.to_vec();
                nested.push(module.ident.clone());
                collect_schema_names(&module.definition, &nested, names);
            }
            Definition::ConstrTypeDcl(constr) => {
                collect_constr_schema_names(constr, module_path, names);
            }
            Definition::TypeDcl(type_dcl) => {
                collect_type_dcl_schema_names(type_dcl, module_path, names);
            }
            Definition::ExceptDcl(except) => {
                names.insert(struct_path(module_path, &except.ident));
            }
            _ => {}
        }
    }
}

fn collect_constr_schema_names(
    constr: &ConstrTypeDcl,
    module_path: &[String],
    names: &mut HashSet<Vec<String>>,
) {
    match constr {
        ConstrTypeDcl::StructDcl(def) => {
            names.insert(struct_path(module_path, &def.ident));
        }
        ConstrTypeDcl::UnionDef(def) => {
            names.insert(struct_path(module_path, &def.ident));
        }
        _ => {}
    }
}

fn collect_type_dcl_schema_names(
    type_dcl: &TypeDcl,
    module_path: &[String],
    names: &mut HashSet<Vec<String>>,
) {
    match type_dcl {
        TypeDcl::ConstrTypeDcl(constr) => {
            collect_constr_schema_names(constr, module_path, names);
        }
        TypeDcl::TypedefDcl(typedef) => {
            for decl in &typedef.decl {
                names.insert(struct_path(module_path, declarator_name(decl)));
            }
        }
        _ => {}
    }
}

fn collect_schema_edges(
    defs: &[Definition],
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    for def in defs {
        match def {
            Definition::ModuleDcl(module) => {
                let mut nested = module_path.to_vec();
                nested.push(module.ident.clone());
                collect_schema_edges(&module.definition, &nested, names, edges);
            }
            Definition::ConstrTypeDcl(constr) => {
                collect_constr_schema_edges(constr, module_path, names, edges);
            }
            Definition::TypeDcl(type_dcl) => {
                collect_type_dcl_schema_edges(type_dcl, module_path, names, edges);
            }
            Definition::ExceptDcl(except) => {
                let owner = join_path(&struct_path(module_path, &except.ident));
                for member in &except.member {
                    push_schema_edges(owner.clone(), module_path, &member.ty, names, edges);
                }
            }
            _ => {}
        }
    }
}

fn collect_constr_schema_edges(
    constr: &ConstrTypeDcl,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    match constr {
        ConstrTypeDcl::StructDcl(def) => {
            let owner = join_path(&struct_path(module_path, &def.ident));
            for member in &def.member {
                push_schema_edges(owner.clone(), module_path, &member.ty, names, edges);
            }
        }
        ConstrTypeDcl::UnionDef(def) => {
            let owner = join_path(&struct_path(module_path, &def.ident));
            for case in &def.case {
                push_union_case_schema_edges(
                    owner.clone(),
                    module_path,
                    &case.element.ty,
                    names,
                    edges,
                );
            }
        }
        _ => {}
    }
}

fn push_union_case_schema_edges(
    owner: String,
    module_path: &[String],
    element: &ElementSpecTy,
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    match element {
        ElementSpecTy::TypeSpec(ty) => {
            push_schema_edges(owner, module_path, ty, names, edges);
        }
        ElementSpecTy::ConstrTypeDcl(ConstrTypeDcl::StructDcl(def)) => {
            for member in &def.member {
                push_schema_edges(owner.clone(), module_path, &member.ty, names, edges);
            }
        }
        ElementSpecTy::ConstrTypeDcl(ConstrTypeDcl::UnionDef(def)) => {
            for case in &def.case {
                push_union_case_schema_edges(
                    owner.clone(),
                    module_path,
                    &case.element.ty,
                    names,
                    edges,
                );
            }
        }
        _ => {}
    }
}

fn collect_type_dcl_schema_edges(
    type_dcl: &TypeDcl,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    match type_dcl {
        TypeDcl::ConstrTypeDcl(constr) => {
            collect_constr_schema_edges(constr, module_path, names, edges);
        }
        TypeDcl::TypedefDcl(typedef) => {
            for decl in &typedef.decl {
                let owner = join_path(&struct_path(module_path, declarator_name(decl)));
                match &typedef.ty {
                    TypedefType::TypeSpec(spec) => {
                        push_schema_edges(owner.clone(), module_path, spec, names, edges);
                    }
                    TypedefType::ConstrTypeDcl(ConstrTypeDcl::StructDcl(def)) => {
                        for member in &def.member {
                            push_schema_edges(owner.clone(), module_path, &member.ty, names, edges);
                        }
                    }
                    TypedefType::ConstrTypeDcl(ConstrTypeDcl::UnionDef(def)) => {
                        for case in &def.case {
                            push_union_case_schema_edges(
                                owner.clone(),
                                module_path,
                                &case.element.ty,
                                names,
                                edges,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn push_schema_edges(
    owner: String,
    module_path: &[String],
    ty: &TypeSpec,
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    for target in schema_targets(module_path, ty, names) {
        edges.push((owner.clone(), join_path(&target)));
    }
}

fn schema_targets(
    module_path: &[String],
    ty: &TypeSpec,
    names: &HashSet<Vec<String>>,
) -> Vec<Vec<String>> {
    let mut targets = Vec::new();
    collect_schema_targets(module_path, ty, names, &mut targets);
    targets
}

fn collect_schema_targets(
    module_path: &[String],
    ty: &TypeSpec,
    names: &HashSet<Vec<String>>,
    targets: &mut Vec<Vec<String>>,
) {
    match ty {
        TypeSpec::ScopedName(name) => {
            if let Some(path) = resolve_struct_path(module_path, name, names) {
                targets.push(path);
            }
        }
        TypeSpec::SequenceType(seq) => {
            collect_schema_targets(module_path, &seq.ty, names, targets);
        }
        TypeSpec::MapType(map) => {
            collect_schema_targets(module_path, &map.key, names, targets);
            collect_schema_targets(module_path, &map.value, names, targets);
        }
        TypeSpec::TemplateType(template) => {
            for arg in &template.args {
                collect_schema_targets(module_path, arg, names, targets);
            }
        }
        _ => {}
    }
}

fn declarator_name(decl: &Declarator) -> &str {
    match decl {
        Declarator::SimpleDeclarator(simple) => &simple.0,
        Declarator::ArrayDeclarator(array) => &array.ident,
    }
}

fn split_path(path: &str) -> Vec<String> {
    path.split("::").map(str::to_string).collect()
}
