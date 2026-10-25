//! Keep inherited references bound to their declaring scope.

use crate::hir::{DeclarationNames, Member, VisitScopedNames};

/// References resolved where an ancestor declared its members.
pub(super) struct MemberBinding<'a> {
    names: &'a DeclarationNames,
    scope: Vec<String>,
}

impl<'a> MemberBinding<'a> {
    /// Anchors inherited references to the ancestor's declaration scope.
    pub(super) fn new(names: &'a DeclarationNames, scope: Vec<String>) -> Self {
        Self { names, scope }
    }

    /// Resolves known names while preserving aliases and external references.
    pub(super) fn bind(&self, member: &mut Member) {
        member.visit_scoped_names(&mut |name| {
            if let Some(resolved) = self.names.resolve(&self.scope, name) {
                *name = resolved;
            }
        });
    }
}
