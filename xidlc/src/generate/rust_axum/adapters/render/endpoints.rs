//! Compose full chains while retaining the original Head outcome on refusal.

use super::*;

impl AdapterRenderer<'_> {
    pub(super) fn endpoints(&self, file: &mut FileContext) -> IdlcResult<()> {
        let Some(head) = self.plan.versions.last() else {
            return Ok(());
        };
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
                    upcast.push(new.function(&format!("{prefix}Request")));
                }
                for step in self.plan.steps[index..].iter().rev() {
                    let old = Names {
                        version: &step.from.id,
                        step_from: &step.from.id,
                        upward: false,
                        types: &step.source_types,
                        layout: &self.layout,
                    };
                    downcast.push(old.function(&format!("{prefix}Response")));
                    downcast_error.push(format!(
                        "downcast_{}_error_to_{}",
                        prefix.to_case(Case::Snake),
                        step.from.id.to_case(Case::Snake)
                    ));
                }
                file.endpoints.push(Endpoint {
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
                });
            }
        }
        self.dispatches(file)?;
        self.type_endpoints(file)
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
                variants.push(DispatchVariant {
                    id: version.version.id.to_case(Case::Pascal),
                    prefix: format!(
                        "{}_from_{}",
                        prefix.to_case(Case::Snake),
                        version.version.id.to_case(Case::Snake)
                    ),
                    response: self.response_type(served, &names)?.0,
                    error: self.error_type(served, &names),
                });
            }
            file.dispatches.push(Dispatch {
                prefix: prefix.to_case(Case::Snake),
                outcome: format!("{prefix}Outcome"),
                request: self.request_type(operation, names.version),
                response: self.response_type(operation, &names)?.0,
                error: self.error_type(operation, &names),
                exhaustive: variants.len() == self.plan.versions.len(),
                variants,
            });
        }
        Ok(())
    }

    fn type_endpoints(&self, file: &mut FileContext) -> IdlcResult<()> {
        let Some(head) = self.plan.versions.last() else {
            return Ok(());
        };
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
                        Names {
                            version: &step.from.id,
                            step_from: &step.from.id,
                            upward: false,
                            types: &step.source_types,
                            layout: &self.layout,
                        }
                        .function(&key)
                    })
                    .collect();
                variants.push(TypeVariant {
                    id: version.version.id.to_case(Case::Pascal),
                    ty: names.named(&key),
                    calls,
                });
            }
            file.types.push(TypeEndpoint {
                name: format!("{}Projection", key.replace("::", "_").to_case(Case::Pascal)),
                function: format!("downcast_{}", key.replace("::", "_").to_case(Case::Snake)),
                head: head_names.named(&key),
                exhaustive: variants.len() == self.plan.versions.len(),
                variants,
            });
        }
        Ok(())
    }
}
