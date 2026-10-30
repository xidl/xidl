//! Render union conversions from the projected shared wire layout.

use super::*;
use crate::generate::adapter::{TypeShape, UnionLayout, UnionShape};

impl AdapterRenderer<'_> {
    pub(super) fn union_conversion(
        &self,
        plan: &ConversionPlan,
        input: &ConversionInput<'_>,
        context: &mut Conversion,
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        let shape = self.union_shape(input)?;
        context.kind = match shape.layout {
            UnionLayout::Http => "http_union",
            UnionLayout::Tagged(_) => "union",
        };
        for copy in &plan.copies {
            context.copies.push(Copy {
                source: copy.source.name.clone(),
                target: copy.target.name.clone(),
                value: input.target.value(&copy.value)?,
                optional: false,
                tags: Self::union_tags(shape, &copy.source.name)?,
            });
        }
        if !plan.consumed.is_empty() {
            let variants = plan
                .consumed
                .iter()
                .map(|member| {
                    let value = input.source.member_type(member)?;
                    let fields = match shape.layout {
                        UnionLayout::Http => vec![(ts_prop_name(&member.name), value)],
                        UnionLayout::Tagged(_) => vec![
                            (
                                "tag".into(),
                                TsType::InlineEnum(Self::union_tags(shape, &member.name)?),
                            ),
                            ("data".into(), value),
                        ],
                    };
                    Ok(TsType::InlineStruct(
                        fields
                            .into_iter()
                            .map(|(prop, ty)| FieldTypeContext {
                                prop,
                                ty,
                                optional: false,
                                doc: Vec::new(),
                            })
                            .collect(),
                    ))
                })
                .collect::<IdlcResult<_>>()?;
            let hook = input.target.hook(input.key);
            file.hooks.push(Hook {
                name: hook.clone(),
                input: TsType::Union(variants),
                output: input.target_type.clone(),
                stored: Vec::new(),
                current: None,
            });
            context.hook = Some(hook);
        }
        Ok(())
    }

    fn union_shape(&self, input: &ConversionInput<'_>) -> IdlcResult<&UnionShape> {
        self.plan
            .versions
            .iter()
            .find(|version| version.version.id == input.source.version)
            .and_then(|version| version.surface.types.get(input.key))
            .and_then(|shape| match shape {
                TypeShape::Union(union) => Some(union),
                _ => None,
            })
            .ok_or_else(|| {
                IdlcError::rpc(format!(
                    "typescript adapters {}: projected union is missing",
                    input.key
                ))
            })
    }

    fn union_tags(shape: &UnionShape, member: &str) -> IdlcResult<Vec<String>> {
        match &shape.layout {
            UnionLayout::Http => Ok(Vec::new()),
            UnionLayout::Tagged(tags) => tags.get(member).cloned().ok_or_else(|| {
                IdlcError::rpc(format!(
                    "typescript adapters union case {member}: wire tags are missing"
                ))
            }),
        }
    }
}
