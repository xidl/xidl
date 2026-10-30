//! Preserve projected union tags across planning, wire decoding and rendering.

use super::{FieldCopy, Member, UnionLayout, UnionShape};
use crate::generate::utils::{has_annotation, union_wire::UnionWire};
use xidl_parser::hir;

impl UnionShape {
    pub(super) fn new(definition: &hir::UnionDef, members: Vec<Member>) -> Self {
        let layout = if has_annotation(&definition.annotations, "http") {
            UnionLayout::Http
        } else {
            UnionLayout::Tagged(
                definition
                    .case
                    .iter()
                    .map(|case| {
                        let name = match &case.element.value {
                            hir::Declarator::SimpleDeclarator(value) => &value.0,
                            hir::Declarator::ArrayDeclarator(value) => &value.ident,
                        };
                        (name.clone(), UnionWire::tags(case))
                    })
                    .collect(),
            )
        };
        Self { members, layout }
    }

    /// A tag change requires an application decision, even if payloads match.
    pub(super) fn preserves_copies(&self, target: &Self, copies: &[FieldCopy]) -> bool {
        match (&self.layout, &target.layout) {
            (UnionLayout::Http, UnionLayout::Http) => true,
            (UnionLayout::Tagged(source), UnionLayout::Tagged(target)) => {
                copies.iter().all(|copy| {
                    match (source.get(&copy.source.name), target.get(&copy.target.name)) {
                        (Some(source), Some(target)) => {
                            source.len() == target.len()
                                && source.iter().all(|tag| target.contains(tag))
                        }
                        _ => false,
                    }
                })
            }
            _ => false,
        }
    }
}
