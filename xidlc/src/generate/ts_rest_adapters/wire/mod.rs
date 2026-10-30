//! Decode served-version wire objects before any adapter hook executes.

mod schema;

use super::names::Names;
use super::wire_context::*;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::wire_layout::WireLayouts;
use crate::generate::adapter::{AdapterPlan, Member, TypeShape, UnionLayout, UnionShape};
use crate::generate::typescript::definition::contexts::{TsType, ZodSchema};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct WireDecoderBuilder<'a> {
    versions: BTreeMap<String, &'a BTreeMap<String, TypeShape>>,
    symbols: BTreeMap<(String, String), String>,
    pending: BTreeSet<(String, String)>,
    definitions: Vec<WireDefinition>,
}

impl<'a> WireDecoderBuilder<'a> {
    pub(super) fn new(plan: &'a AdapterPlan) -> Self {
        let mut versions = BTreeMap::new();
        for item in &plan.versions {
            versions.insert(item.version.id.clone(), &item.surface.types);
        }
        let symbols = versions
            .iter()
            .flat_map(|(version, types)| {
                types.keys().map(move |key| (version.clone(), key.clone()))
            })
            .enumerate()
            .map(|(index, key)| (key, format!("wireType{index}")))
            .collect();
        Self {
            versions,
            symbols,
            pending: BTreeSet::new(),
            definitions: Vec::new(),
        }
    }

    pub(super) fn request(
        &mut self,
        version: &str,
        members: &[Member],
        output: TsType,
    ) -> IdlcResult<String> {
        let name = format!("wireRequest{}Schema", self.definitions.len());
        // HTTP extraction supplies an envelope keyed by source parameter name.
        // Nested values retain their wire keys until this decoder validates them.
        let members = members
            .iter()
            .cloned()
            .map(|mut member| {
                member.wire = member.name.clone();
                member
            })
            .collect::<Vec<_>>();
        let body = self.object(version, &members, BTreeSet::new())?;
        self.definitions.push(WireDefinition {
            name: name.clone(),
            output,
            body: WireBody::Object(body),
        });
        Ok(name)
    }

    pub(super) fn finish(self) -> Vec<WireDefinition> {
        self.definitions
    }

    fn ensure(&mut self, version: &str, key: &str) -> IdlcResult<String> {
        let pair = (version.to_string(), key.to_string());
        let symbol = self
            .symbols
            .get(&pair)
            .cloned()
            .ok_or_else(|| Self::error(key, "wire type is missing"))?;
        if !self.pending.insert(pair) {
            return Ok(symbol);
        }
        let types = self
            .versions
            .get(version)
            .ok_or_else(|| Self::error(version, "wire version is missing"))?;
        let shape = types
            .get(key)
            .cloned()
            .ok_or_else(|| Self::error(key, "wire shape is missing"))?;
        let keys = types.keys().cloned().collect();
        let output = Names {
            keys: &keys,
            version,
            from: version,
            upward: false,
        }
        .named(key);
        let body = match shape {
            TypeShape::Struct(members) => {
                let mut reserved = BTreeSet::new();
                for (other_version, types) in &self.versions {
                    if matches!(types.get(key), Some(TypeShape::Struct(_))) {
                        reserved.extend(
                            self.layouts(other_version)?
                                .layout(key, &mut BTreeSet::new())?
                                .keys,
                        );
                    }
                }
                WireBody::Object(self.object(version, &members, reserved)?)
            }
            TypeShape::Enum(members) => WireBody::Value(if members.is_empty() {
                ZodSchema::Never
            } else {
                ZodSchema::Enum(members.into_iter().map(|member| member.wire).collect())
            }),
            TypeShape::Union(union) => WireBody::Union(self.union_objects(version, &union)?),
            TypeShape::Alias(value) => WireBody::Value(self.schema(version, &value)?),
            TypeShape::Unsupported(reason) => {
                return Err(Self::error(
                    key,
                    &format!("wire decoding does not support {reason}"),
                ));
            }
        };
        self.definitions.push(WireDefinition {
            name: format!("{symbol}Schema"),
            output,
            body,
        });
        Ok(symbol)
    }

    fn union_objects(&mut self, version: &str, union: &UnionShape) -> IdlcResult<Vec<WireObject>> {
        union
            .members
            .iter()
            .map(|member| match &union.layout {
                UnionLayout::Http => {
                    let mut member = member.clone();
                    member.wire = member.name.clone();
                    self.object(version, &[member], BTreeSet::new())
                }
                UnionLayout::Tagged(tags) => {
                    let tags = tags
                        .get(&member.name)
                        .cloned()
                        .ok_or_else(|| Self::error(&member.name, "union case tags are missing"))?;
                    let value = member
                        .value
                        .as_ref()
                        .ok_or_else(|| Self::error(&member.name, "union case type is missing"))?;
                    Ok(WireObject {
                        keys: vec!["tag".into(), "data".into()],
                        forbidden: Vec::new(),
                        catch_all: false,
                        fields: vec![
                            WireField {
                                prop: "tag".into(),
                                optional: false,
                                schema: ZodSchema::Enum(tags),
                                input: WireInput::Named("tag".into()),
                            },
                            WireField {
                                prop: "data".into(),
                                optional: false,
                                schema: self.schema(version, value)?,
                                input: WireInput::Named("data".into()),
                            },
                        ],
                    })
                }
            })
            .collect()
    }

    fn object(
        &mut self,
        version: &str,
        members: &[Member],
        reserved: BTreeSet<String>,
    ) -> IdlcResult<WireObject> {
        let mut fields = Vec::new();
        let mut keys = BTreeSet::new();
        let mut catch_all = false;
        for member in members {
            let value = member
                .value
                .as_ref()
                .ok_or_else(|| Self::error(&member.name, "wire field has no type"))?;
            let input = if member.flatten {
                let layout = self
                    .layouts(version)?
                    .value_layout(value, &mut BTreeSet::new())?;
                if catch_all && layout.catch_all {
                    return Err(Self::error(
                        &member.name,
                        "multiple flattened maps own the same wire keys",
                    ));
                }
                catch_all |= layout.catch_all;
                for key in &layout.keys {
                    if !keys.insert(key.clone()) {
                        return Err(Self::error(key, "flattened wire key is ambiguous"));
                    }
                }
                WireInput::Flatten {
                    keys: layout.keys.into_iter().collect(),
                    receives_rest: layout.catch_all,
                }
            } else {
                if !keys.insert(member.wire.clone()) {
                    return Err(Self::error(&member.wire, "wire key is ambiguous"));
                }
                WireInput::Named(member.wire.clone())
            };
            fields.push(WireField {
                prop: member.name.clone(),
                optional: value.optional,
                schema: self.schema(version, value)?,
                input,
            });
        }
        let forbidden = reserved.difference(&keys).cloned().collect();
        Ok(WireObject {
            keys: keys.into_iter().collect(),
            forbidden,
            catch_all,
            fields,
        })
    }

    fn error(context: &str, reason: &str) -> IdlcError {
        IdlcError::rpc(format!(
            "typescript adapters wire decoder {context}: {reason}"
        ))
    }
    fn layouts<'b>(&'b self, version: &'b str) -> IdlcResult<WireLayouts<'b>> {
        let types = self
            .versions
            .get(version)
            .copied()
            .ok_or_else(|| Self::error(version, "wire surface is missing"))?;
        Ok(WireLayouts { version, types })
    }
}
