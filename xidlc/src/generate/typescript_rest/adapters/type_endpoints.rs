//! Expose complete downcast chains for continuously reachable result types.

use super::context::{Call, FileContext, TypeDispatcher, TypeVariant};
use super::names::Names;
use super::render::AdapterRenderer;
use crate::error::IdlcResult;
use convert_case::{Case, Casing};
use std::collections::BTreeSet;

impl AdapterRenderer<'_> {
    pub(super) fn type_endpoints(&self, file: &mut FileContext) -> IdlcResult<()> {
        let Some(head) = self.plan.versions.last() else {
            return Ok(());
        };
        let mut reachable = self
            .plan
            .steps
            .iter()
            .flat_map(|step| step.types.iter())
            .filter(|ty| ty.downcast.is_some())
            .map(|ty| ty.key.clone())
            .collect::<BTreeSet<_>>();
        reachable.extend(self.plan.head_downcast_types()?);
        let stored = file
            .conversions
            .iter()
            .filter(|conversion| conversion.stored)
            .map(|conversion| conversion.name.clone())
            .collect::<BTreeSet<_>>();
        let head_keys = head.surface.types.keys().cloned().collect();
        let head_names = Names {
            keys: &head_keys,
            version: &head.version.id,
            from: &head.version.id,
            upward: false,
        };
        for key in reachable {
            if !head.surface.types.contains_key(&key) {
                continue;
            }
            let mut variants = Vec::new();
            for (index, version) in self.plan.versions.iter().enumerate() {
                if !version.surface.types.contains_key(&key) {
                    continue;
                }
                let steps = &self.plan.steps[index..];
                if steps.iter().any(|step| {
                    !step
                        .types
                        .iter()
                        .any(|ty| ty.key == key && ty.downcast.is_some())
                }) {
                    continue;
                }
                let keys = version.surface.types.keys().cloned().collect();
                let names = Names {
                    keys: &keys,
                    version: &version.version.id,
                    from: &version.version.id,
                    upward: false,
                };
                let calls = steps
                    .iter()
                    .rev()
                    .map(|step| {
                        let name = Names {
                            keys: &keys,
                            version: &step.from.id,
                            from: &step.from.id,
                            upward: false,
                        }
                        .function(&key);
                        Call {
                            stored: stored.contains(&name),
                            name,
                        }
                    })
                    .collect();
                variants.push(TypeVariant {
                    version: version.version.id.clone(),
                    output: names.named(&key),
                    calls,
                });
            }
            let threads = variants
                .iter()
                .any(|variant: &TypeVariant| variant.calls.iter().any(|call| call.stored));
            file.type_dispatchers.push(TypeDispatcher {
                name: format!("downcast{}", key.replace("::", "_").to_case(Case::Pascal)),
                head: head_names.named(&key),
                variants,
                stored: threads,
            });
        }
        Ok(())
    }
}
