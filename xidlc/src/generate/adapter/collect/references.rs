//! Keep adapter values independent of the module wrapping their wire version.

use super::*;

impl VersionSurface {
    /// Retired values may be rendered in Head storage readers; keep their names
    /// absolute within this surface instead of tied to the old wire module.
    pub(super) fn localize_references(&mut self, version: &str) {
        for shape in self.types.values_mut() {
            match shape {
                TypeShape::Alias(value) => value.localize(version),
                TypeShape::Struct(members)
                | TypeShape::Union(members)
                | TypeShape::Enum(members) => {
                    Self::localize_members(members, version);
                }
                TypeShape::Unsupported(_) => {}
            }
        }
        for operation in self.operations.values_mut() {
            Self::localize_members(&mut operation.inputs, version);
            Self::localize_members(&mut operation.outputs, version);
        }
    }

    fn localize_members(members: &mut [Member], version: &str) {
        for member in members {
            if let Some(value) = &mut member.value {
                value.localize(version);
            }
        }
    }
}

impl TypeUse {
    fn localize(&mut self, version: &str) {
        use hir::VisitScopedNames;
        self.ty.remove_root_namespace(version);
        self.declarator.remove_root_namespace(version);
    }
}
