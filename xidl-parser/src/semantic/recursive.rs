use std::collections::HashSet;

use crate::hir::{
    ConstrTypeDcl, Definition, ModuleDcl, ScopedName, Specification, StructDcl, TypeDcl, TypeSpec,
    TypedefType, UnionDef,
};
use crate::semantic::recursive_graph::collect_recursive_edges;

pub(crate) fn annotate_recursive_members(spec: &mut Specification) {
    let mut names = HashSet::new();
    collect_struct_names(&spec.0, &[], &mut names);
    let mut edges = Vec::new();
    collect_struct_edges(&spec.0, &[], &names, &mut edges);
    let recursive_edges = collect_recursive_edges(&edges);
    apply_recursive_flags(&mut spec.0, &[], &names, &recursive_edges);
}

fn collect_struct_names(
    defs: &[Definition],
    module_path: &[String],
    names: &mut HashSet<Vec<String>>,
) {
    for def in defs {
        match def {
            Definition::ModuleDcl(module) => {
                let mut nested = module_path.to_vec();
                nested.push(module.ident.clone());
                collect_struct_names(&module.definition, &nested, names);
            }
            Definition::ConstrTypeDcl(constr) => collect_constr_names(constr, module_path, names),
            Definition::TypeDcl(type_dcl) => collect_type_dcl_names(type_dcl, module_path, names),
            _ => {}
        }
    }
}

fn collect_constr_names(
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

fn collect_type_dcl_names(
    type_dcl: &TypeDcl,
    module_path: &[String],
    names: &mut HashSet<Vec<String>>,
) {
    match type_dcl {
        TypeDcl::ConstrTypeDcl(constr) => collect_constr_names(constr, module_path, names),
        TypeDcl::TypedefDcl(typedef) => match &typedef.ty {
            TypedefType::ConstrTypeDcl(ConstrTypeDcl::StructDcl(def)) => {
                names.insert(struct_path(module_path, &def.ident));
            }
            TypedefType::ConstrTypeDcl(ConstrTypeDcl::UnionDef(def)) => {
                names.insert(struct_path(module_path, &def.ident));
            }
            _ => {}
        },
        TypeDcl::NativeDcl(_) => {}
    }
}

fn collect_struct_edges(
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
                collect_struct_edges(&module.definition, &nested, names, edges);
            }
            Definition::ConstrTypeDcl(constr) => {
                collect_constr_edges(constr, module_path, names, edges);
            }
            Definition::TypeDcl(type_dcl) => {
                collect_type_dcl_edges(type_dcl, module_path, names, edges);
            }
            _ => {}
        }
    }
}

fn collect_constr_edges(
    constr: &ConstrTypeDcl,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    match constr {
        ConstrTypeDcl::StructDcl(def) => {
            collect_struct_def_edges(def, module_path, names, edges);
        }
        ConstrTypeDcl::UnionDef(def) => {
            collect_union_def_edges(def, module_path, names, edges);
        }
        _ => {}
    }
}

fn collect_type_dcl_edges(
    type_dcl: &TypeDcl,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    match type_dcl {
        TypeDcl::ConstrTypeDcl(constr) => collect_constr_edges(constr, module_path, names, edges),
        TypeDcl::TypedefDcl(typedef) => match &typedef.ty {
            TypedefType::ConstrTypeDcl(ConstrTypeDcl::StructDcl(def)) => {
                collect_struct_def_edges(def, module_path, names, edges);
            }
            TypedefType::ConstrTypeDcl(ConstrTypeDcl::UnionDef(def)) => {
                collect_union_def_edges(def, module_path, names, edges);
            }
            _ => {}
        },
        TypeDcl::NativeDcl(_) => {}
    }
}

fn collect_struct_def_edges(
    def: &StructDcl,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    let owner = join_path(&struct_path(module_path, &def.ident));
    for member in &def.member {
        let Some(target) = direct_struct_target(module_path, &member.ty, names) else {
            continue;
        };
        edges.push((owner.clone(), join_path(&target)));
    }
}

fn collect_union_def_edges(
    def: &UnionDef,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    edges: &mut Vec<(String, String)>,
) {
    let owner = join_path(&struct_path(module_path, &def.ident));
    for case in &def.case {
        let ty = match &case.element.ty {
            crate::hir::ElementSpecTy::TypeSpec(spec) => spec,
            _ => continue,
        };
        let Some(target) = direct_struct_target(module_path, ty, names) else {
            continue;
        };
        edges.push((owner.clone(), join_path(&target)));
    }
}

fn apply_recursive_flags(
    defs: &mut [Definition],
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    recursive_edges: &HashSet<(String, String)>,
) {
    for def in defs {
        match def {
            Definition::ModuleDcl(ModuleDcl {
                ident, definition, ..
            }) => {
                let mut nested = module_path.to_vec();
                nested.push(ident.clone());
                apply_recursive_flags(definition, &nested, names, recursive_edges);
            }
            Definition::ConstrTypeDcl(constr) => {
                apply_constr_recursive_flags(constr, module_path, names, recursive_edges);
            }
            Definition::TypeDcl(type_dcl) => {
                apply_type_dcl_recursive_flags(type_dcl, module_path, names, recursive_edges);
            }
            _ => {}
        }
    }
}

fn apply_constr_recursive_flags(
    constr: &mut ConstrTypeDcl,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    recursive_edges: &HashSet<(String, String)>,
) {
    match constr {
        ConstrTypeDcl::StructDcl(def) => {
            apply_struct_recursive_flags(def, module_path, names, recursive_edges);
        }
        ConstrTypeDcl::UnionDef(def) => {
            apply_union_recursive_flags(def, module_path, names, recursive_edges);
        }
        _ => {}
    }
}

fn apply_type_dcl_recursive_flags(
    type_dcl: &mut TypeDcl,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    recursive_edges: &HashSet<(String, String)>,
) {
    match type_dcl {
        TypeDcl::ConstrTypeDcl(constr) => {
            apply_constr_recursive_flags(constr, module_path, names, recursive_edges);
        }
        TypeDcl::TypedefDcl(typedef) => match &mut typedef.ty {
            TypedefType::ConstrTypeDcl(ConstrTypeDcl::StructDcl(def)) => {
                apply_struct_recursive_flags(def, module_path, names, recursive_edges);
            }
            TypedefType::ConstrTypeDcl(ConstrTypeDcl::UnionDef(def)) => {
                apply_union_recursive_flags(def, module_path, names, recursive_edges);
            }
            _ => {}
        },
        TypeDcl::NativeDcl(_) => {}
    }
}

fn apply_struct_recursive_flags(
    def: &mut StructDcl,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    recursive_edges: &HashSet<(String, String)>,
) {
    let owner = join_path(&struct_path(module_path, &def.ident));
    for member in &mut def.member {
        member.recursive = direct_struct_target(module_path, &member.ty, names)
            .map(|target| recursive_edges.contains(&(owner.clone(), join_path(&target))))
            .unwrap_or(false);
    }
}

fn apply_union_recursive_flags(
    def: &mut UnionDef,
    module_path: &[String],
    names: &HashSet<Vec<String>>,
    recursive_edges: &HashSet<(String, String)>,
) {
    let owner = join_path(&struct_path(module_path, &def.ident));
    for case in &mut def.case {
        let ty = match &case.element.ty {
            crate::hir::ElementSpecTy::TypeSpec(spec) => spec,
            _ => continue,
        };
        case.element.recursive = direct_struct_target(module_path, ty, names)
            .map(|target| recursive_edges.contains(&(owner.clone(), join_path(&target))))
            .unwrap_or(false);
    }
}

fn direct_struct_target(
    module_path: &[String],
    ty: &TypeSpec,
    names: &HashSet<Vec<String>>,
) -> Option<Vec<String>> {
    match ty {
        TypeSpec::ScopedName(name) => resolve_struct_path(module_path, name, names),
        _ => None,
    }
}

fn resolve_struct_path(
    module_path: &[String],
    scoped_name: &ScopedName,
    names: &HashSet<Vec<String>>,
) -> Option<Vec<String>> {
    if scoped_name.is_root {
        let path = scoped_name.name.clone();
        return names.contains(&path).then_some(path);
    }

    for depth in (0..=module_path.len()).rev() {
        let mut candidate = module_path[..depth].to_vec();
        candidate.extend(scoped_name.name.iter().cloned());
        if names.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn struct_path(module_path: &[String], ident: &str) -> Vec<String> {
    let mut path = module_path.to_vec();
    path.push(ident.to_string());
    path
}

fn join_path(path: &[String]) -> String {
    path.join("::")
}
