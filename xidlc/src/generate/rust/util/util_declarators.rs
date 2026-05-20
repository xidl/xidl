use crate::generate::rust::util::{render_const, rust_ident, rust_type};
use serde_json::json;
use xidl_parser::hir;

pub fn array_type(base: &str, dims: &[String]) -> String {
    let mut out = base.to_string();
    for dim in dims.iter().rev() {
        out = format!("[{}; {}]", out, dim);
    }
    out
}

pub fn declarator_dims(decl: &hir::Declarator) -> Vec<String> {
    match decl {
        hir::Declarator::SimpleDeclarator(_) => Vec::new(),
        hir::Declarator::ArrayDeclarator(value) => {
            value.len.iter().map(|len| render_const(&len.0)).collect()
        }
    }
}

pub fn declarator_name(decl: &hir::Declarator) -> String {
    match decl {
        hir::Declarator::SimpleDeclarator(value) => value.0.clone(),
        hir::Declarator::ArrayDeclarator(value) => value.ident.clone(),
    }
}

pub fn type_with_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> String {
    let base = rust_type(ty);
    let dims = declarator_dims(decl);
    if dims.is_empty() {
        base
    } else {
        array_type(&base, &dims)
    }
}

pub fn typedef_json(base: &str, decl: &hir::Declarator) -> serde_json::Value {
    let name = rust_ident(&declarator_name(decl));
    let dims = declarator_dims(decl);
    let ty = if dims.is_empty() {
        base.to_string()
    } else {
        array_type(base, &dims)
    };
    json!({ "ty": ty, "name": name })
}

pub fn constr_type_scoped_name(constr: &hir::ConstrTypeDcl) -> hir::ScopedName {
    let ident = match constr {
        hir::ConstrTypeDcl::StructForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::StructDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::EnumDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionDef(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitsetDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitmaskDcl(def) => def.ident.clone(),
    };
    hir::ScopedName {
        name: vec![ident],
        is_root: false,
    }
}
