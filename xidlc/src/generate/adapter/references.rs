//! Direction-aware reference effects, kept outside the canonical map core.

use super::model::*;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::scope::resolve_canonical;
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::hir;

pub(super) struct ReferencePlanner<'a> {
    pub surface: &'a VersionSurface,
    pub paired: &'a BTreeSet<String>,
    pub context: &'a str,
}

impl ReferencePlanner<'_> {
    pub(super) fn value(&self, value: &TypeUse) -> IdlcResult<ValuePlan> {
        let mut plan = if value.representation == ValueRepresentation::ByteBuffer {
            ValuePlan::Identity
        } else {
            self.ty(&value.ty, &value.scope)?
        };
        if let hir::Declarator::ArrayDeclarator(array) = &value.declarator {
            for _ in &array.len {
                plan = ValuePlan::Array(Box::new(plan));
            }
        }
        if value.optional {
            plan = ValuePlan::Optional(Box::new(plan));
        }
        Ok(plan)
    }

    fn ty(&self, ty: &hir::TypeSpec, scope: &[String]) -> IdlcResult<ValuePlan> {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                let key =
                    resolve_canonical(scope, name, |key| self.surface.types.contains_key(key));
                match key {
                    Some(key) if self.paired.contains(&key) => Ok(ValuePlan::Named(key)),
                    Some(key) => Err(IdlcError::rpc(format!(
                        "version adapters {}: copied reference {key} has no declaration in both versions",
                        self.context
                    ))),
                    None => Err(IdlcError::rpc(format!(
                        "version adapters {}: cannot resolve copied type {}",
                        self.context,
                        name.name.join("::")
                    ))),
                }
            }
            hir::TypeSpec::SequenceType(sequence) => {
                Ok(ValuePlan::Sequence(Box::new(self.ty(&sequence.ty, scope)?)))
            }
            hir::TypeSpec::MapType(map) => Ok(ValuePlan::Map {
                key: Box::new(self.ty(&map.key, scope)?),
                value: Box::new(self.ty(&map.value, scope)?),
            }),
            hir::TypeSpec::TemplateType(template) => {
                for argument in &template.args {
                    self.ty(argument, scope)?;
                }
                Err(IdlcError::rpc(format!(
                    "version adapters {}: template {} has no declared value-conversion semantics",
                    self.context, template.ident
                )))
            }
            _ => Ok(ValuePlan::Identity),
        }
    }

    pub(super) fn copies(&self, plan: &mut ConversionPlan) -> IdlcResult<()> {
        for copy in &mut plan.copies {
            copy.value = match &copy.source.value {
                Some(value) => self.value(value)?,
                None => ValuePlan::Identity,
            };
        }
        Ok(())
    }

    pub(super) fn reachable(
        conversions: &BTreeMap<String, TypeConversion>,
        roots: &[ValuePlan],
    ) -> BTreeSet<String> {
        let mut queue = Vec::new();
        for root in roots {
            root.references(&mut queue);
        }
        let mut seen = BTreeSet::new();
        while let Some(key) = queue.pop() {
            if !seen.insert(key.clone()) {
                continue;
            }
            if let Some(conversion) = conversions.get(&key) {
                match conversion {
                    TypeConversion::Struct(plan)
                    | TypeConversion::Enum(plan)
                    | TypeConversion::Union(plan) => {
                        for copy in &plan.copies {
                            copy.value.references(&mut queue);
                        }
                    }
                    TypeConversion::Alias(value) => value.references(&mut queue),
                    TypeConversion::Hook => {}
                }
            }
        }
        seen
    }
}

impl ValuePlan {
    fn references(&self, output: &mut Vec<String>) {
        match self {
            Self::Named(key) => output.push(key.clone()),
            Self::Sequence(value) | Self::Optional(value) | Self::Array(value) => {
                value.references(output)
            }
            Self::Map { key, value } => {
                key.references(output);
                value.references(output);
            }
            Self::Identity => {}
        }
    }
}

impl AdapterPlan {
    /// Head identity projections also exist when there are no adjacent steps.
    pub(crate) fn head_downcast_types(&self) -> IdlcResult<BTreeSet<String>> {
        let Some(head) = self.versions.last() else {
            return Ok(BTreeSet::new());
        };
        let paired = head.surface.types.keys().cloned().collect();
        let references = ReferencePlanner {
            surface: &head.surface,
            paired: &paired,
            context: "Head outputs",
        };
        let mut pending = Vec::new();
        for operation in head.surface.operations.values() {
            for member in &operation.outputs {
                if let Some(value) = &member.value {
                    references.value(value)?.references(&mut pending);
                }
            }
            pending.extend(operation.raises.iter().cloned());
        }
        let mut reached = BTreeSet::new();
        while let Some(key) = pending.pop() {
            if !reached.insert(key.clone()) {
                continue;
            }
            match head.surface.types.get(&key) {
                Some(TypeShape::Struct(members) | TypeShape::Union(members)) => {
                    for member in members {
                        if let Some(value) = &member.value {
                            references.value(value)?.references(&mut pending);
                        }
                    }
                }
                Some(TypeShape::Alias(value)) => references.value(value)?.references(&mut pending),
                _ => {}
            }
        }
        Ok(reached)
    }
}
