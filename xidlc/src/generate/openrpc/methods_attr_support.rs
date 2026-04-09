use std::collections::HashSet;
use xidl_parser::hir;

pub(super) fn validate_attr_collision(
    user_ops: &HashSet<String>,
    attr_name: &str,
    getter: &str,
    setter: Option<&str>,
) {
    let getter_conflict = user_ops.contains(getter);
    let setter_conflict = setter.map(|name| user_ops.contains(name)).unwrap_or(false);
    if getter_conflict || setter_conflict {
        let setter_text = setter
            .map(|value| format!(" or `{value}`"))
            .unwrap_or_default();
        panic!(
            "attribute `{attr_name}` conflicts with user-defined operation `{getter}`{setter_text}"
        );
    }
}

pub(super) fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}
