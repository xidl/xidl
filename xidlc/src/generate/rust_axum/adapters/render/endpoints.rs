//! Compose full chains while retaining the original Head outcome on refusal.

use super::*;

impl AdapterRenderer<'_> {
    pub(super) fn endpoints(&self, file: &mut FileContext) -> IdlcResult<()> {
        let Some(head) = self.plan.versions.last() else {
            return Ok(());
        };
        let stored = Self::stored_conversions(file);
        let shapes = file
            .conversions
            .iter()
            .map(|conversion| {
                (
                    conversion.name.clone(),
                    (conversion.current, conversion.pair),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let stored_operations = file
            .operations
            .iter()
            .filter(|operation| operation.stored)
            .map(|operation| operation.prefix.replace("_from_", "_error_to_"))
            .collect::<BTreeSet<_>>();
        let head_names = Names {
            version: &head.version.id,
            step_from: &head.version.id,
            upward: false,
            types: &head.surface.types,
            layout: &self.layout,
        };
        for (index, version) in self.plan.versions.iter().enumerate() {
            let names = Names {
                version: &version.version.id,
                step_from: &version.version.id,
                upward: false,
                types: &version.surface.types,
                layout: &self.layout,
            };
            for (key, operation) in &version.surface.operations {
                let head_operation = head
                    .surface
                    .operations
                    .get(key)
                    .ok_or_else(|| Layout::error(key, "cannot serve retired operation"))?;
                if head_operation.http.meta.stream.kind.is_some()
                    || head_operation.http.meta.upgrade_protocol.is_some()
                {
                    return Err(Layout::error(
                        key,
                        "stream and upgrade frame adapters belong to phase 2",
                    ));
                }
                let prefix = Self::prefix(operation);
                let mark = self
                    .plan
                    .updates
                    .operations
                    .get(key)
                    .map(|update| head_names.named(&update.return_key));
                let update = self.plan.updates.operations.contains_key(key);
                let stored_type = self
                    .plan
                    .updates
                    .operations
                    .get(key)
                    .filter(|update| !update.stored.is_empty())
                    .map(|_| crate::generate::rust_axum::adapters::names::Names::stored_type(key));
                let mut upcast = Vec::new();
                let mut downcast = Vec::new();
                let mut downcast_error = Vec::new();
                for step in &self.plan.steps[index..] {
                    let new = Names {
                        version: &step.to.id,
                        step_from: &step.from.id,
                        upward: true,
                        types: &step.target_types,
                        layout: &self.layout,
                    };
                    let request = new.function(&format!("{prefix}Request"));
                    let (current, pair) = shapes.get(&request).copied().unwrap_or_default();
                    upcast.push(Call {
                        name: request,
                        stored: false,
                        current,
                        pair,
                    });
                }
                for step in self.plan.steps[index..].iter().rev() {
                    let old = Names {
                        version: &step.from.id,
                        step_from: &step.from.id,
                        upward: false,
                        types: &step.source_types,
                        layout: &self.layout,
                    };
                    let response = old.function(&format!("{prefix}Response"));
                    downcast.push(Call {
                        stored: stored.contains(&response),
                        name: response,
                        current: false,
                        pair: false,
                    });
                    let error = format!(
                        "{}_error_to_{}",
                        prefix.to_case(Case::Snake),
                        step.from.id.to_case(Case::Snake)
                    );
                    downcast_error.push(Call {
                        stored: stored_operations.contains(&error),
                        name: format!("downcast_{error}"),
                        current: false,
                        pair: false,
                    });
                }
                let threads = downcast
                    .iter()
                    .chain(&downcast_error)
                    .any(|call: &Call| call.stored);
                file.endpoints.push(Endpoint {
                    current: update && !upcast.is_empty(),
                    current_type: mark.clone(),
                    prefix: format!(
                        "{}_from_{}",
                        prefix.to_case(Case::Snake),
                        version.version.id.to_case(Case::Snake)
                    ),
                    version_id: version.version.id.to_case(Case::Pascal),
                    request: self.request_type(operation, names.version),
                    head_request: self.request_type(head_operation, head_names.version),
                    response: self.response_type(operation, &names)?.0,
                    head_response: self.response_type(head_operation, &head_names)?.0,
                    error: self.error_type(operation, &names),
                    head_error: self.error_type(head_operation, &head_names),
                    upcast,
                    downcast,
                    downcast_error,
                    parameters: Vec::new(),
                    stored: threads,
                    stored_type: stored_type.clone(),
                });
            }
        }
        self.dispatches(file)?;
        self.type_endpoints(file)
    }

    /// Names of conversions that take the stored-values parameter.
    fn stored_conversions(file: &FileContext) -> BTreeSet<String> {
        file.conversions
            .iter()
            .filter(|conversion| conversion.stored)
            .map(|conversion| conversion.name.clone())
            .collect()
    }

    fn dispatches(&self, file: &mut FileContext) -> IdlcResult<()> {
        let Some(head) = self.plan.versions.last() else {
            return Ok(());
        };
        let names = Names {
            version: &head.version.id,
            step_from: &head.version.id,
            upward: false,
            types: &head.surface.types,
            layout: &self.layout,
        };
        let stored_endpoints = file
            .endpoints
            .iter()
            .filter(|endpoint| endpoint.stored)
            .map(|endpoint| endpoint.prefix.clone())
            .collect::<BTreeSet<_>>();
        let current_endpoints = file
            .endpoints
            .iter()
            .filter(|endpoint| endpoint.current)
            .map(|endpoint| endpoint.prefix.clone())
            .collect::<BTreeSet<_>>();
        for (key, operation) in &head.surface.operations {
            let prefix = Self::prefix(operation);
            let mut variants = Vec::new();
            for version in &self.plan.versions {
                let Some(served) = version.surface.operations.get(key) else {
                    continue;
                };
                let names = Names {
                    version: &version.version.id,
                    step_from: &version.version.id,
                    upward: false,
                    types: &version.surface.types,
                    layout: &self.layout,
                };
                let variant_prefix = format!(
                    "{}_from_{}",
                    prefix.to_case(Case::Snake),
                    version.version.id.to_case(Case::Snake)
                );
                variants.push(DispatchVariant {
                    id: version.version.id.to_case(Case::Pascal),
                    stored: stored_endpoints.contains(&variant_prefix),
                    current: current_endpoints.contains(&variant_prefix),
                    prefix: variant_prefix,
                    response: self.response_type(served, &names)?.0,
                    error: self.error_type(served, &names),
                });
            }
            let threads = variants
                .iter()
                .any(|variant: &DispatchVariant| variant.stored);
            let update = self.plan.updates.operations.get(key);
            file.dispatches.push(Dispatch {
                prefix: prefix.to_case(Case::Snake),
                outcome: format!("{prefix}Outcome"),
                request: self.request_type(operation, names.version),
                response: self.response_type(operation, &names)?.0,
                error: self.error_type(operation, &names),
                exhaustive: variants.len() == self.plan.versions.len(),
                variants,
                stored: threads,
                stored_type: update
                    .filter(|update| !update.stored.is_empty())
                    .map(|_| crate::generate::rust_axum::adapters::names::Names::stored_type(key)),
                current: update.is_some(),
                current_type: update.map(|update| names.named(&update.return_key)),
            });
        }
        Ok(())
    }

    fn type_endpoints(&self, file: &mut FileContext) -> IdlcResult<()> {
        let Some(head) = self.plan.versions.last() else {
            return Ok(());
        };
        let stored = Self::stored_conversions(file);
        let mut reachable = self.plan.head_downcast_types()?;
        reachable.extend(
            self.plan
                .steps
                .iter()
                .flat_map(|step| step.types.iter())
                .filter(|ty| ty.downcast.is_some())
                .map(|ty| ty.key.clone()),
        );
        for key in reachable {
            if !head.surface.types.contains_key(&key) {
                continue;
            }
            self.layout
                .validate(&format!("{}::{key}", head.version.id))?;
            let head_names = Names {
                version: &head.version.id,
                step_from: &head.version.id,
                upward: false,
                types: &head.surface.types,
                layout: &self.layout,
            };
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
                let names = Names {
                    version: &version.version.id,
                    step_from: &version.version.id,
                    upward: false,
                    types: &version.surface.types,
                    layout: &self.layout,
                };
                let calls = steps
                    .iter()
                    .rev()
                    .map(|step| {
                        let call_name = Names {
                            version: &step.from.id,
                            step_from: &step.from.id,
                            upward: false,
                            types: &step.source_types,
                            layout: &self.layout,
                        }
                        .function(&key);
                        Call {
                            stored: stored.contains(&call_name),
                            name: call_name,
                            current: false,
                            pair: false,
                        }
                    })
                    .collect();
                variants.push(TypeVariant {
                    id: version.version.id.to_case(Case::Pascal),
                    ty: names.named(&key),
                    calls,
                });
            }
            let threads = variants
                .iter()
                .any(|variant: &TypeVariant| variant.calls.iter().any(|call| call.stored));
            file.types.push(TypeEndpoint {
                name: format!("{}Projection", key.replace("::", "_").to_case(Case::Pascal)),
                function: format!("downcast_{}", key.replace("::", "_").to_case(Case::Snake)),
                head: head_names.named(&key),
                exhaustive: variants.len() == self.plan.versions.len(),
                variants,
                stored: threads,
            });
        }
        Ok(())
    }
}
