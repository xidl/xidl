use std::collections::HashSet;

use crate::hir::ScopedName;

pub(super) fn resolve_struct_path(
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

pub(super) fn struct_path(module_path: &[String], ident: &str) -> Vec<String> {
    let mut path = module_path.to_vec();
    path.push(ident.to_string());
    path
}

pub(super) fn join_path(path: &[String]) -> String {
    path.join("::")
}
