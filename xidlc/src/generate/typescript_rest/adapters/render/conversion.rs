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
        let mark = input.update.clone().unwrap_or_default();
        let mut context = Conversion {
            name: target.function(key),
            kind: "struct",
            source: input.source_type.clone(),
            target: input.target_type.clone(),
            hook: None,
            hook_stored: false,
            stored: false,
            current: mark.current,
            current_type: mark.head.clone(),
            hook_current: false,
            pair: mark.pair,
            stored_type: mark.stored_type.clone(),
            stored_field: false,
            copies: Vec::new(),
            assignments: Vec::new(),
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
                    stored: Vec::new(),
                    current: None,
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
                    stored: Vec::new(),
                    current: None,
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
                        stored: Vec::new(),
                        current: None,
                    });
                    context.hook = Some(hook);
                }
            } else {
                self.decision(plan, &input, &mark, &mut context, file)?;
            }
        }
        file.conversions.push(context);
        Ok(())
    }

    /// Upcast additions stay optional decisions; downcast removals are always
    /// decided from stored values or an explicit `OMIT`, never dropped. Update
    /// upcasts also receive the Head resource and persist paired members.
    fn decision(
        &self,
        plan: &ConversionPlan,
        input: &ConversionInput<'_>,
        mark: &UpdateMark,
        context: &mut Conversion,
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        if plan.assignments.is_empty() && plan.ignored.is_empty() {
            return Ok(());
        }
        let downward = !input.target.upward;
        let mut fields = Vec::new();
        let mut keys = BTreeSet::new();
        let mut readers = Vec::new();
        for member in &plan.assignments {
            let key = ts_prop_name(&member.name);
            keys.insert(member.name.clone());
            let optional = member.value.as_ref().is_some_and(Names::optional);
            let ty = input.target.member_type(member)?;
            if downward {
                let reader = Names::reader(input.key, &member.name);
                file.readers.push(Reader {
                    name: reader.clone(),
                    input: input.source_type.clone(),
                    output: TsType::Optional(Box::new(ty.clone())),
                    wire: member.wire.clone(),
                    version: input.target.version.into(),
                });
                readers.push(reader);
            }
            let omittable = downward && optional;
            fields.push(Field {
                key: key.clone(),
                ty: if omittable {
                    TsType::Union(vec![ty, TsType::ScopedName("Omitted".into())])
                } else {
                    ty
                },
                optional: optional && !downward,
            });
            context.assignments.push(Assignment {
                source: member.name.clone(),
                target: member.name.clone(),
                optional: optional && !downward,
                omittable,
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
        if let Some(bag) = mark
            .stored_type
            .as_ref()
            .filter(|_| !mark.paired.is_empty())
        {
            if !keys.insert("stored".into()) {
                return Err(IdlcError::rpc(format!(
                    "typescript adapters {}: a member named stored collides with the persisted bag",
                    input.key
                )));
            }
            fields.push(Field {
                key: "stored".into(),
                ty: TsType::ScopedName(bag.clone()),
                optional: false,
            });
            context.stored_field = true;
        }
        let decision = input.target.decision(input.key);
        let hook = input.target.hook(input.key);
        file.decisions.push(Decision {
            name: decision.clone(),
            fields,
        });
        context.hook_stored = !readers.is_empty();
        context.stored = context.hook_stored;
        context.hook_current = mark.current;
        file.hooks.push(Hook {
            name: hook.clone(),
            input: input.source_type.clone(),
            output: TsType::ScopedName(decision),
            stored: readers,
            current: mark.head.clone().filter(|_| mark.current),
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
    pub(super) update: Option<UpdateMark>,
}
