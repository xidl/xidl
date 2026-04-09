use crate::generate::cpp::util::{cpp_scoped_name, cpp_type};
use serde_json::json;
use xidl_parser::hir;

pub(super) fn parent_names(def: &hir::InterfaceDef) -> Vec<String> {
    def.header
        .parent
        .as_ref()
        .map(|value| {
            value
                .0
                .iter()
                .map(|parent| cpp_scoped_name(&parent.0))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn render_op(op: &hir::OpDcl) -> serde_json::Value {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => "void".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => cpp_type(ty),
    };
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut param_list = Vec::new();
    for param in params {
        let ty = render_param_type(&param.ty, param.attr.as_ref());
        param_list.push(format!("{ty} {}", &param.declarator.0));
    }
    json!({ "ret": ret, "name": &op.ident, "params": param_list, "is_const": false })
}

pub(super) fn render_attr(attr: &hir::AttrDcl) -> Vec<serde_json::Value> {
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .map(|name| {
                json!({
                    "ret": attr_return_type(&spec.ty),
                    "name": name,
                    "params": Vec::<String>::new(),
                    "is_const": true,
                })
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        push_accessor_methods(&mut out, decl.0.as_str(), &spec.ty);
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    push_accessor_methods(&mut out, declarator.0.as_str(), &spec.ty);
                }
            }
            out
        }
    }
}

fn push_accessor_methods(out: &mut Vec<serde_json::Value>, name: &str, ty: &hir::TypeSpec) {
    let ret = attr_return_type(ty);
    let param = render_param_type(ty, None);
    out.push(json!({
        "ret": ret, "name": name, "params": Vec::<String>::new(), "is_const": true,
    }));
    out.push(json!({
        "ret": "void", "name": name, "params": vec![format!("{param} value")], "is_const": false,
    }));
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn attr_return_type(ty: &hir::TypeSpec) -> String {
    if is_value_type(ty) {
        cpp_type(ty)
    } else {
        format!("const {}&", cpp_type(ty))
    }
}

fn render_param_type(ty: &hir::TypeSpec, attr: Option<&hir::ParamAttribute>) -> String {
    match attr.map(|value| value.0.as_str()) {
        Some("out") | Some("inout") => format!("{}&", cpp_type(ty)),
        _ if is_value_type(ty) => cpp_type(ty),
        _ => format!("const {}&", cpp_type(ty)),
    }
}

fn is_value_type(ty: &hir::TypeSpec) -> bool {
    matches!(
        ty,
        hir::TypeSpec::SimpleTypeSpec(
            hir::SimpleTypeSpec::IntegerType(_)
                | hir::SimpleTypeSpec::FloatingPtType
                | hir::SimpleTypeSpec::CharType
                | hir::SimpleTypeSpec::WideCharType
                | hir::SimpleTypeSpec::Boolean
        )
    )
}
