//! Assemble adjacent request, response, and error conversions.

use super::*;

impl AdapterRenderer<'_> {
    pub(super) fn operation(
        &self,
        step: &StepPlan,
        plan: &OperationPlan,
        names: (&Names<'_>, &Names<'_>),
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        let (old, new) = names;
        let source = &plan.source;
        let target = &plan.target;
        let interface = source.interface.split("::").last().ok_or_else(|| {
            IdlcError::rpc(format!(
                "typescript adapters: invalid interface {}",
                source.interface
            ))
        })?;
        let prefix = method_struct_prefix(interface, &source.name);
        let scope_prefix = source.scope.join("_");
        let function_prefix = if scope_prefix.is_empty() {
            prefix.clone()
        } else {
            format!("{scope_prefix}_{prefix}")
        };
        let request_key = format!("{function_prefix}Request");
        let request_source = if source.inputs.is_empty() {
            self.object_type(&source.inputs, old)?
        } else {
            old.operation_type(&source.interface, &source.name, "Request")
        };
        let request_target = if target.inputs.is_empty() {
            self.object_type(&target.inputs, new)?
        } else {
            new.operation_type(&target.interface, &target.name, "Request")
        };
        self.conversion(
            &TypeConversion::Struct(plan.upcast.clone()),
            ConversionInput {
                key: &request_key,
                target: new,
                source: old,
                source_type: request_source.clone(),
                target_type: request_target.clone(),
            },
            file,
        )?;
        let response_key = format!("{function_prefix}Response");
        self.conversion(
            &TypeConversion::Struct(plan.downcast.clone()),
            ConversionInput {
                key: &response_key,
                target: old,
                source: new,
                source_type: self.object_type(&target.outputs, new)?,
                target_type: self.object_type(&source.outputs, old)?,
            },
            file,
        )?;
        let result_type = |members: &[Member], names: &Names<'_>| -> IdlcResult<TsType> {
            match members {
                [] => Ok(TsType::Void),
                [member] if member.name == "return" => names.member_type(member),
                _ => self.object_type(members, names),
            }
        };
        let mut schema_path = vec![step.from.id.clone()];
        schema_path.extend(source.scope.clone());
        let schema = (!source.inputs.is_empty())
            .then(|| scoped_name(&schema_path, &format!("{prefix}RequestSchema")));
        let errors_source = target
            .raises
            .iter()
            .map(|key| self.error_class(key, &step.to.id))
            .collect::<IdlcResult<Vec<_>>>()?;
        let errors_target = source
            .raises
            .iter()
            .map(|key| self.error_class(key, &step.from.id))
            .collect::<IdlcResult<Vec<_>>>()?;
        let mut error_arms = Vec::new();
        for key in &target.raises {
            let class = self.error_class(key, &step.to.id)?;
            if source.raises.contains(key) {
                error_arms.push(ErrorArm {
                    class,
                    target_class: Some(self.error_class(key, &step.from.id)?),
                    call: Some(old.function(key)),
                    hook: None,
                });
            } else {
                let hook = old.hook(&format!("{function_prefix}_{key}Error"));
                let output = TsType::Union(
                    errors_target
                        .iter()
                        .map(|class| TsType::ScopedName(format!("errors.{class}")))
                        .chain(std::iter::once(TsType::ScopedName("FrameworkError".into())))
                        .collect(),
                );
                file.hooks.push(Hook {
                    name: hook.clone(),
                    input: TsType::ScopedName(format!("errors.{class}")),
                    output,
                });
                error_arms.push(ErrorArm {
                    class,
                    target_class: None,
                    call: None,
                    hook: Some(hook),
                });
            }
        }
        file.operations.push(Operation {
            prefix: function_prefix,
            from: step.from.id.to_case(Case::Pascal),
            version: step.from.value.clone(),
            type_path: plan.key.clone(),
            request_source,
            request_target,
            schema,
            request_call: new.function(&request_key),
            result_source: result_type(&target.outputs, new)?,
            result_target: result_type(&source.outputs, old)?,
            result_call: Some(old.function(&response_key)),
            errors_source,
            errors_target,
            error_arms,
            result_source_direct: target.outputs.len() == 1 && target.outputs[0].name == "return",
            result_target_direct: source.outputs.len() == 1 && source.outputs[0].name == "return",
        });
        Ok(())
    }
}
