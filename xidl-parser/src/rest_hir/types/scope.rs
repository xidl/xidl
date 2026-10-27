use super::TypeDeclarations;
use crate::hir;

impl TypeDeclarations<'_> {
    /// Resolves a (module path, identifier) pair for a scoped name written
    /// inside `module_path`, walking outward through enclosing modules.
    pub(in crate::rest_hir) fn resolve_scoped_definition<'a, T>(
        items: impl Iterator<Item = (&'a [String], &'a T)>,
        module_path: &[String],
        scoped: &hir::ScopedName,
        ident_of: impl Fn(&T) -> &str,
    ) -> Option<&'a T> {
        let ident = scoped.name.last()?;
        let prefix = &scoped.name[..scoped.name.len() - 1];
        let items: Vec<(&'a [String], &'a T)> = items.collect();
        if scoped.is_root {
            return items
                .into_iter()
                .find(|(path, item)| *path == prefix && ident_of(item) == &ident[..])
                .map(|(_, item)| item);
        }
        for skip in (0..=module_path.len()).rev() {
            let scope = module_path[..skip]
                .iter()
                .chain(prefix)
                .cloned()
                .collect::<Vec<_>>();
            if let Some(found) = items
                .iter()
                .find(|(path, item)| *path == scope.as_slice() && ident_of(item) == &ident[..])
                .map(|(_, item)| *item)
            {
                return Some(found);
            }
        }
        None
    }
}
