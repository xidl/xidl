//! Versioned include path selector parsing.

/// Splits an include or import path at its trailing `@<version>` selector.
///
/// A leading `@` belongs to the path, so `"@xidl/http"` keeps its name, and a
/// path without a usable selector keeps the `@` it was written with.
pub fn split_version_selector(path: &str) -> (&str, Option<&str>) {
    let Some(index) = path.rfind('@').filter(|index| *index > 0) else {
        return (path, None);
    };
    let (target, selector) = path.split_at(index);
    let selector = selector.trim_start_matches('@');
    if selector.is_empty() {
        return (path, None);
    }
    (target, Some(selector))
}
