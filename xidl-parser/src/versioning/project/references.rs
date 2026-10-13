//! Reference collection and root-qualification rewriting.

use crate::hir::{ConstType, InterfaceDef, ScopedName, SwitchTypeSpec, TypeSpec};

pub(super) fn touch_type(ty: &mut TypeSpec, references: &mut Vec<(String, String)>, decl: &str) {
    match ty {
        TypeSpec::ScopedName(name) => touch_scoped(name, references, decl),
        TypeSpec::SequenceType(sequence) => touch_type(&mut sequence.ty, references, decl),
        TypeSpec::MapType(map) => {
            touch_type(&mut map.key, references, decl);
            touch_type(&mut map.value, references, decl);
        }
        TypeSpec::TemplateType(template) => {
            for arg in &mut template.args {
                touch_type(arg, references, decl);
            }
        }
        _ => {}
    }
}

pub(super) fn touch_const_type(
    ty: &mut ConstType,
    references: &mut Vec<(String, String)>,
    decl: &str,
) {
    match ty {
        ConstType::ScopedName(name) => touch_scoped(name, references, decl),
        ConstType::SequenceType(sequence) => touch_type(&mut sequence.ty, references, decl),
        _ => {}
    }
}

pub(super) fn touch_switch_type(
    ty: &mut SwitchTypeSpec,
    references: &mut Vec<(String, String)>,
    decl: &str,
) {
    if let SwitchTypeSpec::ScopedName(name) = ty {
        touch_scoped(name, references, decl);
    }
}

/// Clears root qualification: every user declaration moves inside the version
/// module, so `::Name` must resolve relative to it.
pub(super) fn touch_scoped(
    name: &mut ScopedName,
    references: &mut Vec<(String, String)>,
    decl: &str,
) {
    name.is_root = false;
    if let Some(last) = name.name.last() {
        references.push((decl.to_string(), last.clone()));
    }
}

pub(super) fn touch_names(
    names: &mut [ScopedName],
    references: &mut Vec<(String, String)>,
    decl: &str,
) {
    for name in names {
        touch_scoped(name, references, decl);
    }
}

pub(super) fn touch_interface_parent(
    def: &mut InterfaceDef,
    references: &mut Vec<(String, String)>,
    decl: &str,
) {
    if let Some(parent) = &mut def.header.parent {
        for crate::hir::InterfaceName(name) in &mut parent.0 {
            touch_scoped(name, references, decl);
        }
    }
}
