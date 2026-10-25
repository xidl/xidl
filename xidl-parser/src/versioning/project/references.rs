//! Reference collection and version-local root qualification.

use super::projector::Projector;
use crate::hir::{
    ConstExpr, ConstType, Declarator, ScopedName, SwitchTypeSpec, TypeSpec, VisitScopedNames,
};

/// One reference from a surviving declaration to a named declaration.
#[derive(Debug)]
pub(super) struct Reference {
    /// Declaration path holding the reference, e.g. `Pet::owner`.
    pub(super) decl: String,
    /// Reference segments as written, e.g. `["models", "Owner"]`.
    pub(super) segments: Vec<String>,
    /// Whether the source explicitly selected the document root.
    pub(super) is_root: bool,
}

impl Reference {
    /// Renders the reference the way it appears in IDL source.
    pub(super) fn rendered(&self) -> String {
        let path = self.segments.join("::");
        if self.is_root {
            format!("::{path}")
        } else {
            path
        }
    }
}

impl Projector<'_> {
    pub(super) fn touch_type(&mut self, ty: &mut TypeSpec, decl: &str) {
        ty.visit_scoped_names(&mut |name| self.touch_scoped(name, decl));
    }

    pub(super) fn touch_const_type(&mut self, ty: &mut ConstType, decl: &str) {
        ty.visit_scoped_names(&mut |name| self.touch_scoped(name, decl));
    }

    pub(super) fn touch_switch_type(&mut self, ty: &mut SwitchTypeSpec, decl: &str) {
        if let SwitchTypeSpec::ScopedName(name) = ty {
            self.touch_scoped(name, decl);
        }
    }

    /// Rewrites a constant expression and records the names it references.
    pub(super) fn touch_const_expr(&mut self, expr: &mut ConstExpr, decl: &str) {
        expr.visit_scoped_names(&mut |name| self.touch_scoped(name, decl));
    }

    /// Rewrites array declarators, including their bounds.
    pub(super) fn touch_declarators(&mut self, declarators: &mut [Declarator], decl: &str) {
        for declarator in declarators {
            declarator.visit_scoped_names(&mut |name| self.touch_scoped(name, decl));
        }
    }

    /// Records the source reference before moving local root names into this version.
    pub(super) fn touch_scoped(&mut self, name: &mut ScopedName, decl: &str) {
        if !name.name.is_empty() {
            self.references.push(Reference {
                decl: decl.to_string(),
                segments: name.name.clone(),
                is_root: name.is_root,
            });
        }
        self.qualify_root(name);
    }

    /// Keeps local absolute names absolute after wrapping declarations in a version.
    pub(super) fn qualify_root(&self, name: &mut ScopedName) {
        if name.is_root && self.declarations.resolve(&[], name).is_some() {
            name.name.insert(0, self.def.ident.clone());
        }
    }
}
