//! Build Rust model constructors while preserving the declared base composition.

use super::context::Conversion;
use super::layout::{Declaration, Layout};
use crate::error::IdlcResult;
use crate::generate::rust::util::{declarator_name, rust_ident};
use crate::generate::utils::scope::resolve_canonical;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
pub(super) struct Construction {
    pub ty: String,
    pub base: Option<Box<Self>>,
    pub fields: Vec<ConstructionField>,
}

#[derive(Serialize)]
pub(super) struct ConstructionField {
    pub name: String,
    pub value: ConstructionValue,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub(super) enum ConstructionValue {
    Projected { index: usize },
    Decision { name: String, wrap: bool },
    Absent,
    Decoded { index: usize },
}

impl Construction {
    pub(super) fn flat(ty: String, fields: Vec<ConstructionField>) -> Self {
        Self {
            ty,
            fields,
            base: None,
        }
    }
}

impl Conversion {
    pub(super) fn construction_fields(&self) -> Vec<ConstructionField> {
        let copies = self
            .copies
            .iter()
            .enumerate()
            .map(|(index, copy)| ConstructionField {
                name: copy.target.clone(),
                value: ConstructionValue::Projected { index },
            });
        let assignments = self.assignments.iter().map(|assignment| ConstructionField {
            name: assignment.name.clone(),
            value: ConstructionValue::Decision {
                name: assignment.name.clone(),
                wrap: assignment.wrap,
            },
        });
        let absences = self.absences.iter().map(|name| ConstructionField {
            name: name.clone(),
            value: ConstructionValue::Absent,
        });
        copies.chain(assignments).chain(absences).collect()
    }
}

impl Layout<'_> {
    pub(super) fn parent(&self, key: &str) -> IdlcResult<Option<String>> {
        let Some(Declaration::Struct(value)) = self.declarations.get(key) else {
            return Ok(None);
        };
        let Some(parent) = value.parent.first() else {
            return Ok(None);
        };
        let mut scope = key.split("::").map(str::to_string).collect::<Vec<_>>();
        scope.pop();
        resolve_canonical(&scope, parent, |name| self.declarations.contains_key(name))
            .map(Some)
            .ok_or_else(|| Self::error(key, "base model layout is missing"))
    }

    pub(super) fn construction(
        &self,
        key: &str,
        fields: Vec<ConstructionField>,
    ) -> IdlcResult<Construction> {
        let declared = self
            .members(key)?
            .iter()
            .flat_map(|member| &member.ident)
            .map(|declarator| rust_ident(&declarator_name(declarator)))
            .collect::<BTreeSet<_>>();
        let (own, inherited): (Vec<_>, Vec<_>) = fields
            .into_iter()
            .partition(|field| declared.contains(&field.name));
        if own.len() != declared.len() {
            return Err(Self::error(key, "model constructor has unassigned fields"));
        }
        let base = match self.parent(key)? {
            Some(parent) => Some(Box::new(self.construction(&parent, inherited)?)),
            None if inherited.is_empty() => None,
            None => return Err(Self::error(key, "model constructor has undeclared fields")),
        };
        Ok(Construction {
            ty: key
                .split("::")
                .map(rust_ident)
                .collect::<Vec<_>>()
                .join("::"),
            base,
            fields: own,
        })
    }
}
