//! Assemble resolved-version entry points that compose the adjacent conversions.

use super::context::{Dispatcher, Endpoint, FileContext};
use super::names::Names;
use super::render::AdapterRenderer;
use super::wire::WireDecoderBuilder;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::{Member, Operation};
use crate::generate::typescript::definition::contexts::TsType;
use crate::generate::typescript::definition::names::method_struct_prefix;
use convert_case::{Case, Casing};
use std::collections::{BTreeMap, BTreeSet};

impl AdapterRenderer<'_> {
    pub(super) fn endpoints(&self, file: &mut FileContext) -> IdlcResult<()> {
        let head = self
            .plan
            .versions
            .last()
            .ok_or_else(|| IdlcError::rpc("typescript adapters: version timeline is empty"))?;
        let head_keys = head.surface.types.keys().cloned().collect();
        let head_names = Names {
            keys: &head_keys,
            version: &head.version.id,
            from: &head.version.id,
            upward: true,
        };
        let mut wire = WireDecoderBuilder::new(self.plan);
        let mut dispatchers: BTreeMap<String, Dispatcher> = BTreeMap::new();
        for (index, version) in self.plan.versions.iter().enumerate() {
            let keys = version.surface.types.keys().cloned().collect();
            let names = Names {
                keys: &keys,
                version: &version.version.id,
                from: &version.version.id,
                upward: false,
            };
            for (key, operation) in &version.surface.operations {
                let head_operation = head.surface.operations.get(key).ok_or_else(|| {
                    IdlcError::rpc(format!(
                        "typescript adapters: operation {key} is absent from Head"
                    ))
                })?;
                if head_operation.http.meta.stream.kind.is_some()
                    || head_operation.http.meta.upgrade_protocol.is_some()
                {
                    return Err(IdlcError::rpc(format!(
                        "typescript adapters {key}: stream and upgrade frame adapters belong to phase 2"
                    )));
                }
                let interface = operation.interface.split("::").last().ok_or_else(|| {
                    IdlcError::rpc(format!("typescript adapters: invalid operation {key}"))
                })?;
                let base = method_struct_prefix(interface, &operation.name);
                let prefix = if operation.scope.is_empty() {
                    base
                } else {
                    format!("{}_{}", operation.scope.join("_"), base)
                };
                let request = self.request_type(operation, &names)?;
                let result = self.result_type(&operation.outputs, &names)?;
                let head_request = self.request_type(head_operation, &head_names)?;
                let head_result = self.result_type(&head_operation.outputs, &head_names)?;
                let errors = operation
                    .raises
                    .iter()
                    .map(|key| self.error_class(key, &version.version.id))
                    .collect::<IdlcResult<Vec<_>>>()?;
                let head_errors = head_operation
                    .raises
                    .iter()
                    .map(|key| self.error_class(key, &head.version.id))
                    .collect::<IdlcResult<Vec<_>>>()?;
                let steps = self.plan.steps[index..]
                    .iter()
                    .map(|step| {
                        file.operations
                            .iter()
                            .find(|operation| {
                                operation.type_path == *key && operation.version == step.from.value
                            })
                            .cloned()
                            .ok_or_else(|| {
                                IdlcError::rpc(format!(
                                    "typescript adapters {key}: missing adjacent operation in {}",
                                    step.from.id
                                ))
                            })
                    })
                    .collect::<IdlcResult<_>>()?;
                let from = version.version.id.to_case(Case::Pascal);
                let schema =
                    wire.request(&version.version.id, &operation.inputs, request.clone())?;
                let dispatcher = dispatchers
                    .entry(key.clone())
                    .or_insert_with(|| Dispatcher {
                        prefix: prefix.clone(),
                        head_request: head_request.clone(),
                        head_result: head_result.clone(),
                        head_errors: head_errors.clone(),
                        versions: Vec::new(),
                        results: Vec::new(),
                        errors: Vec::new(),
                    });
                dispatcher
                    .versions
                    .push((version.version.id.clone(), from.clone()));
                dispatcher.results.push(result.clone());
                dispatcher.errors.extend(errors.iter().cloned());
                file.endpoints.push(Endpoint {
                    prefix,
                    from,
                    id: version.version.id.clone(),
                    version: version.version.value.clone(),
                    type_path: key.clone(),
                    schema,
                    request,
                    result,
                    head_request,
                    head_result,
                    errors,
                    head_errors,
                    steps,
                });
            }
        }
        for dispatcher in dispatchers.values_mut() {
            dispatcher.errors = std::mem::take(&mut dispatcher.errors)
                .into_iter()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
        }
        file.dispatchers = dispatchers.into_values().collect();
        file.wire_definitions = wire.finish();
        Ok(())
    }

    fn request_type(&self, operation: &Operation, names: &Names<'_>) -> IdlcResult<TsType> {
        if operation.inputs.is_empty() {
            self.object_type(&[], names)
        } else {
            Ok(names.operation_type(&operation.interface, &operation.name, "Request"))
        }
    }

    fn result_type(&self, members: &[Member], names: &Names<'_>) -> IdlcResult<TsType> {
        match members {
            [] => Ok(TsType::Void),
            [member] if member.name == "return" => names.member_type(member),
            _ => self.object_type(members, names),
        }
    }
}
