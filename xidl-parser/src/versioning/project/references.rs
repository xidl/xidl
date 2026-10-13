//! Reference collection and root-qualification rewriting.

use crate::hir::{
    ConstExpr, ConstType, Declarator, InterfaceDef, ScopedName, SwitchTypeSpec, TypeSpec,
};

/// One reference from a surviving declaration to a named declaration.
#[derive(Debug)]
pub(super) struct Reference {
    /// Declaration path holding the reference, e.g. `Pet::owner`.
    pub(super) decl: String,
    /// Reference segments as written, e.g. `["models", "Owner"]`.
    pub(super) segments: Vec<String>,
}

impl Reference {
    /// Renders the reference the way it appears in IDL source.
    pub(super) fn rendered(&self) -> String {
        self.segments.join("::")
    }
}

pub(super) fn touch_type(ty: &mut TypeSpec, references: &mut Vec<Reference>, decl: &str) {
    match ty {
        TypeSpec::ScopedName(name) => touch_scoped(name, references, decl),
        TypeSpec::SequenceType(sequence) => {
            touch_type(&mut sequence.ty, references, decl);
            if let Some(len) = &mut sequence.len {
                touch_const_expr(&mut len.0, references, decl);
            }
        }
        TypeSpec::MapType(map) => {
            touch_type(&mut map.key, references, decl);
            touch_type(&mut map.value, references, decl);
            if let Some(len) = &mut map.len {
                touch_const_expr(&mut len.0, references, decl);
            }
        }
        TypeSpec::TemplateType(template) => {
            for arg in &mut template.args {
                touch_type(arg, references, decl);
            }
        }
        _ => {}
    }
}

pub(super) fn touch_const_type(ty: &mut ConstType, references: &mut Vec<Reference>, decl: &str) {
    match ty {
        ConstType::ScopedName(name) => touch_scoped(name, references, decl),
        ConstType::SequenceType(sequence) => touch_type(&mut sequence.ty, references, decl),
        _ => {}
    }
}

pub(super) fn touch_switch_type(
    ty: &mut SwitchTypeSpec,
    references: &mut Vec<Reference>,
    decl: &str,
) {
    if let SwitchTypeSpec::ScopedName(name) = ty {
        touch_scoped(name, references, decl);
    }
}

/// Rewrites a constant expression and records the names it references.
pub(super) fn touch_const_expr(expr: &mut ConstExpr, references: &mut Vec<Reference>, decl: &str) {
    match expr {
        ConstExpr::ScopedName(name) => touch_scoped(name, references, decl),
        ConstExpr::UnaryExpr(_, value) => touch_const_expr(value, references, decl),
        ConstExpr::BinaryExpr(_, left, right) => {
            touch_const_expr(left, references, decl);
            touch_const_expr(right, references, decl);
        }
        ConstExpr::Literal(_) => {}
    }
}

/// Rewrites array declarators, including their bounds.
pub(super) fn touch_declarators(
    declarators: &mut [Declarator],
    references: &mut Vec<Reference>,
    decl: &str,
) {
    for declarator in declarators {
        if let Declarator::ArrayDeclarator(array) = declarator {
            for len in &mut array.len {
                touch_const_expr(&mut len.0, references, decl);
            }
        }
    }
}

/// Clears root qualification: every user declaration moves inside the version
/// module, so `::Name` must resolve relative to it.
pub(super) fn touch_scoped(name: &mut ScopedName, references: &mut Vec<Reference>, decl: &str) {
    name.is_root = false;
    if !name.name.is_empty() {
        references.push(Reference {
            decl: decl.to_string(),
            segments: name.name.clone(),
        });
    }
}

/// Relativizes a `raises(...)` list without registering references.
///
/// Exception sets are the one position without an annotation slot: an entry
/// whose exception this version removed is pruned instead of reported.
pub(super) fn relativize_names(names: &mut [ScopedName]) {
    for name in names {
        name.is_root = false;
    }
}

pub(super) fn touch_interface_parent(
    def: &mut InterfaceDef,
    references: &mut Vec<Reference>,
    decl: &str,
) {
    if let Some(parent) = &mut def.header.parent {
        for crate::hir::InterfaceName(name) in &mut parent.0 {
            touch_scoped(name, references, decl);
        }
    }
}
