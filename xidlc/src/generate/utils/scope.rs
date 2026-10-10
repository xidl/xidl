use xidl_parser::hir::ScopedName;

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
