//! Render model conversions and explicit business decisions.

use super::*;

impl AdapterRenderer<'_> {
    pub(super) fn conversion(
        &self,
        plan: &TypeConversion,
        input: ConversionInput<'_>,
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        let key = input.key;
        let target = input.target;
        let mut context = Conversion {
            name: target.function(key),
            kind: "struct",
            source: input.source_type.clone(),
            target: input.target_type.clone(),
            hook: None,
            copies: Vec::new(),
            assignments: Vec::new(),
            absent_keys: Vec::new(),
            shared_values: Vec::new(),
            value: None,
        };
        let plan = match plan {
            TypeConversion::Hook => {
                context.kind = "hook";
                let hook = target.hook(key);
                file.hooks.push(Hook {
                    name: hook.clone(),
                    input: input.source_type,
                    output: input.target_type,
                });
                context.hook = Some(hook);
                file.conversions.push(context);
                return Ok(());
            }
            TypeConversion::Alias(value) => {
                context.kind = "alias";
                context.value = Some(target.value(value)?);
                file.conversions.push(context);
                return Ok(());
            }
            TypeConversion::Enum(plan) => {
                context.kind = "enum";
                plan
            }
            TypeConversion::Union(plan) => {
                context.kind = "union";
                plan
            }
            TypeConversion::Struct(plan) => plan,
        };
        if context.kind == "enum" {
            context.shared_values = plan
                .copies
                .iter()
                .map(|copy| copy.source.wire.clone())
                .collect();
            if !plan.consumed.is_empty() {
                let name = target.hook(key);
                file.hooks.push(Hook {
                    name: name.clone(),
                    input: TsType::InlineEnum(
                        plan.consumed
                            .iter()
                            .map(|member| member.wire.clone())
                            .collect(),
                    ),
                    output: input.target_type,
                });
                context.hook = Some(name);
            }
        } else {
            for copy in &plan.copies {
                context.copies.push(Copy {
                    source: copy.source.name.clone(),
                    target: copy.target.name.clone(),
                    value: target.value(&copy.value)?,
                    optional: copy.target.value.as_ref().is_some_and(Names::optional),
                });
            }
            context.absent_keys = plan
                .absences
                .iter()
                .map(|member| member.name.clone())
                .collect();
            if context.kind == "union" {
                if !plan.consumed.is_empty() {
                    let hook = target.hook(key);
                    let variants = plan
                        .consumed
                        .iter()
                        .map(|member| {
                            Ok(TsType::InlineStruct(vec![FieldTypeContext {
                                prop: ts_prop_name(&member.name),
                                ty: input.source.member_type(member)?,
                                optional: false,
                                doc: Vec::new(),
                            }]))
                        })
                        .collect::<IdlcResult<Vec<_>>>()?;
                    file.hooks.push(Hook {
                        name: hook.clone(),
                        input: TsType::Union(variants),
                        output: input.target_type.clone(),
                    });
                    context.hook = Some(hook);
                }
            } else {
                self.decision(plan, &input, &mut context, file)?;
            }
        }
        file.conversions.push(context);
        Ok(())
    }

    fn decision(
        &self,
        plan: &ConversionPlan,
        input: &ConversionInput<'_>,
        context: &mut Conversion,
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        if plan.assignments.is_empty() && plan.ignored.is_empty() {
            return Ok(());
        }
        let mut fields = Vec::new();
        let mut keys = BTreeSet::new();
        for member in &plan.assignments {
            let key = ts_prop_name(&member.name);
            keys.insert(member.name.clone());
            fields.push(Field {
                key: key.clone(),
                ty: input.target.member_type(member)?,
                optional: member.value.as_ref().is_some_and(Names::optional),
            });
            context.assignments.push(Assignment {
                source: member.name.clone(),
                target: member.name.clone(),
                optional: member.value.as_ref().is_some_and(Names::optional),
            });
        }
        for member in &plan.ignored {
            let key = if keys.contains(&member.name) {
                format!("ignored${}", member.name)
            } else {
                member.name.clone()
            };
            if !keys.insert(key.clone()) {
                return Err(IdlcError::rpc(format!(
                    "typescript adapters {}: ignored acknowledgement {key} is ambiguous",
                    input.key
                )));
            }
            fields.push(Field {
                key: ts_prop_name(&key),
                ty: TsType::Primitive("Ignored".into()),
                optional: false,
            });
        }
        let decision = input.target.decision(input.key);
        let hook = input.target.hook(input.key);
        file.decisions.push(Decision {
            name: decision.clone(),
            fields,
        });
        file.hooks.push(Hook {
            name: hook.clone(),
            input: input.source_type.clone(),
            output: TsType::ScopedName(decision),
        });
        context.hook = Some(hook);
        Ok(())
    }
}

pub(super) struct ConversionInput<'a> {
    pub(super) key: &'a str,
    pub(super) target: &'a Names<'a>,
    pub(super) source: &'a Names<'a>,
    pub(super) source_type: TsType,
    pub(super) target_type: TsType,
}
