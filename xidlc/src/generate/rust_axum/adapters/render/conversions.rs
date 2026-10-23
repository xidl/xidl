//! Render mechanical member copies, variant subsets, and explicit decisions.

use super::*;
use crate::generate::adapter::ConversionPlan;

impl AdapterRenderer<'_> {
    pub(super) fn conversion(
        &self,
        plan: &TypeConversion,
        input: ConversionInput<'_>,
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        if let Some(key) = input.declaration {
            self.layout
                .validate(&format!("{}::{key}", input.source.version))?;
            self.layout
                .validate(&format!("{}::{key}", input.target.version))?;
        }
        let mark = input.update.clone().unwrap_or_default();
        let mut output = Conversion {
            name: input.target.function(input.key),
            source: input.source_type.clone(),
            target: input.target_type.clone(),
            kind: "struct",
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
            absences: Vec::new(),
            variants: Vec::new(),
            changed: Vec::new(),
            changed_type: format!(
                "{}Changes",
                input.target.function(input.key).to_case(Case::Pascal)
            ),
            value: None,
            source_shape: input.source_shape,
            target_shape: input.target_shape,
            version: input.source.version.into(),
            upward: input.target.upward,
            type_path: input.key.into(),
        };
        let members = match plan {
            TypeConversion::Hook => {
                output.kind = "hook";
                output.hook = Some(input.target.hook(input.key));
                file.hooks.push(Hook {
                    name: input.target.hook(input.key),
                    source: input.source_type,
                    target: input.target_type,
                    stored: false,
                    current: None,
                });
                file.conversions.push(output);
                return Ok(());
            }
            TypeConversion::Alias(value) => {
                output.kind = "alias";
                output.value = Some(input.target.value(value));
                file.conversions.push(output);
                return Ok(());
            }
            TypeConversion::Enum(members) => {
                output.kind = "enum";
                members
            }
            TypeConversion::Union(members) => {
                output.kind = "union";
                members
            }
            TypeConversion::Struct(members) => members,
        };
        if output.kind == "enum" {
            self.enumeration(members, &input, &mut output, file)?;
        } else if output.kind == "union" {
            self.union(members, &input, &mut output, file)?;
        } else {
            for copy in &members.copies {
                let boxed = match input.declaration {
                    Some(key) => (
                        self.layout.recursive(
                            &format!("{}::{key}", input.source.version),
                            &copy.source.name,
                        )?,
                        self.layout.recursive(
                            &format!("{}::{key}", input.target.version),
                            &copy.target.name,
                        )?,
                    ),
                    None => (false, false),
                };
                output.copies.push(Copy {
                    source: rust_ident(&copy.source.name),
                    target: rust_ident(&copy.target.name),
                    value: input.target.field_value(&copy.value, boxed),
                });
            }
            output.absences = members
                .absences
                .iter()
                .map(|member| rust_ident(&member.name))
                .collect();
            self.decision(members, &input, &mark, &mut output, file)?;
        }
        file.conversions.push(output);
        Ok(())
    }

    /// Upcast additions stay optional decisions; downcast removals are always
    /// decided from stored values or an explicit `None`, never dropped.
    fn decision(
        &self,
        plan: &ConversionPlan,
        input: &ConversionInput<'_>,
        mark: &UpdateMark,
        output: &mut Conversion,
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        if plan.assignments.is_empty() && plan.ignored.is_empty() {
            return Ok(());
        }
        let downward = !input.target.upward;
        let mut fields = Vec::new();
        let mut keys = BTreeSet::new();
        for member in &plan.assignments {
            let name = rust_ident(&member.name);
            keys.insert(name.clone());
            let ty = self.field_type(member, input.target, input.declaration)?;
            if downward {
                let optional = member.value.as_ref().is_some_and(|value| value.optional);
                file.readers.push(Reader {
                    name: Names::reader(input.key, &member.name),
                    source: input.source_type.clone(),
                    output: if optional {
                        ty.clone()
                    } else {
                        format!("Option<{ty}>")
                    },
                    wire: member.wire.clone(),
                    version: input.target.version.into(),
                });
            }
            fields.push(Field {
                name: name.clone(),
                ty,
            });
            output.assignments.push(name);
        }
        for member in &plan.ignored {
            let name = rust_ident(&member.name);
            let name = if keys.contains(&name) {
                format!("ignored_{}", member.name)
            } else {
                name
            };
            if !keys.insert(name.clone()) {
                return Err(Layout::error(
                    input.key,
                    "ignored acknowledgement field is ambiguous",
                ));
            }
            fields.push(Field {
                name,
                ty: "Ignored".into(),
            });
        }
        if let Some(bag) = mark
            .stored_type
            .as_ref()
            .filter(|_| !mark.paired.is_empty())
        {
            if !keys.insert("stored".into()) {
                return Err(Layout::error(
                    input.key,
                    "a member named stored collides with the persisted bag",
                ));
            }
            fields.push(Field {
                name: "stored".into(),
                ty: bag.clone(),
            });
            output.stored_field = true;
        }
        let decision = input.target.decision(input.key);
        let name = input.target.hook(input.key);
        file.decisions.push(Decision {
            name: decision.clone(),
            fields,
        });
        output.hook_stored = downward && !plan.assignments.is_empty();
        output.stored = output.hook_stored;
        output.hook_current = mark.current;
        file.hooks.push(Hook {
            name: name.clone(),
            source: input.source_type.clone(),
            target: decision,
            stored: output.hook_stored,
            current: mark.head.clone().filter(|_| mark.current),
        });
        output.hook = Some(name);
        Ok(())
    }

    fn enumeration(
        &self,
        plan: &ConversionPlan,
        input: &ConversionInput<'_>,
        output: &mut Conversion,
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        output.variants = plan
            .copies
            .iter()
            .map(|copy| rust_ident(&copy.source.name))
            .collect();
        output.changed = plan
            .consumed
            .iter()
            .map(|member| ChangedVariant {
                name: rust_ident(&member.name),
                ty: None,
            })
            .collect();
        if !output.changed.is_empty() {
            let name = input.target.hook(input.key);
            file.hooks.push(Hook {
                name: name.clone(),
                source: output.changed_type.clone(),
                target: input.target_type.clone(),
                stored: false,
                current: None,
            });
            output.hook = Some(name);
        }
        Ok(())
    }
}
