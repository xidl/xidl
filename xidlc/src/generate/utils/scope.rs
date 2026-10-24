use xidl_parser::hir::ScopedName;

use crate::generate::rust::util::rust_ident;

/// Build ordered IDL scope candidates for a type reference.
pub fn canonical_candidates(scope: &[String], name: &ScopedName) -> Vec<String> {
    if name.is_root {
        return vec![name.name.join("::")];
    }
    let mut out = Vec::new();
    for len in (0..=scope.len()).rev() {
        let mut parts = scope[..len].to_vec();
        parts.extend(name.name.iter().cloned());
        out.push(parts.join("::"));
    }
    out
}

/// Resolve a scoped reference against existing canonical names.
pub fn resolve_canonical(
    scope: &[String],
    name: &ScopedName,
    exists: impl Fn(&str) -> bool,
) -> Option<String> {
    canonical_candidates(scope, name)
        .into_iter()
        .find(|candidate| exists(candidate.as_str()))
}

/// Rust path for a canonical type path as seen from the rendering module,
/// climbing `super::` past the scope levels the two paths do not share, so
/// the result resolves at any nesting depth.
pub fn relative_path(output: &[String], canonical: &str) -> String {
    let target = canonical.split("::").collect::<Vec<_>>();
    let common = output
        .iter()
        .zip(&target[..target.len() - 1])
        .take_while(|(a, b)| a.as_str() == **b)
        .count();
    std::iter::repeat_n("super".to_string(), output.len() - common)
        .chain(target[common..].iter().map(|part| rust_ident(part)))
        .collect::<Vec<_>>()
        .join("::")
}
