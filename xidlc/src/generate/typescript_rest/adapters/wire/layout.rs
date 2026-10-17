//! Assign promoted wire keys to flattened fields.

use super::*;
use crate::generate::adapter::TypeUse;
use crate::generate::utils::scope::resolve_canonical;
use xidl_parser::hir;

#[derive(Default)]
pub(super) struct WireLayout {
    pub keys: BTreeSet<String>,
    pub catch_all: bool,
}

impl WireDecoderBuilder<'_> {
    pub(super) fn layout(
        &self,
        version: &str,
        key: &str,
        visiting: &mut BTreeSet<String>,
    ) -> IdlcResult<WireLayout> {
        if !visiting.insert(key.to_string()) {
            return Err(Self::error(key, "flattened reference cycle"));
        }
        let shape = self
            .versions
            .get(version)
            .and_then(|types| types.get(key))
            .ok_or_else(|| Self::error(key, "flattened type is missing"))?;
        let layout = match shape {
            TypeShape::Struct(members) => {
                let mut layout = WireLayout::default();
                for member in members {
                    let next = if member.flatten {
                        self.value_layout(
                            version,
                            member.value.as_ref().ok_or_else(|| {
                                Self::error(&member.name, "flattened member has no type")
                            })?,
                            visiting,
                        )?
                    } else {
                        WireLayout {
                            keys: BTreeSet::from([member.wire.clone()]),
                            catch_all: false,
                        }
                    };
                    if layout.catch_all && next.catch_all {
                        return Err(Self::error(key, "multiple flattened catchall fields"));
                    }
                    layout.catch_all |= next.catch_all;
                    for wire in next.keys {
                        if !layout.keys.insert(wire.clone()) {
                            return Err(Self::error(&wire, "flattened wire key is ambiguous"));
                        }
                    }
                }
                layout
            }
            TypeShape::Alias(value) => self.value_layout(version, value, visiting)?,
            _ => return Err(Self::error(key, "flatten requires a struct or map")),
        };
        visiting.remove(key);
        Ok(layout)
    }

    pub(super) fn value_layout(
        &self,
        version: &str,
        value: &TypeUse,
        visiting: &mut BTreeSet<String>,
    ) -> IdlcResult<WireLayout> {
        if matches!(value.declarator, hir::Declarator::ArrayDeclarator(_)) {
            return Err(Self::error(version, "flatten does not support arrays"));
        }
        match &value.ty {
            hir::TypeSpec::MapType(_)
            | hir::TypeSpec::AnyType
            | hir::TypeSpec::ObjectType
            | hir::TypeSpec::ValueBaseType => Ok(WireLayout {
                keys: BTreeSet::new(),
                catch_all: true,
            }),
            hir::TypeSpec::ScopedName(name) => self.layout(
                version,
                &self.resolve(version, &value.scope, name)?,
                visiting,
            ),
            _ => Err(Self::error(version, "flatten requires a struct or map")),
        }
    }

    pub(super) fn resolve(
        &self,
        version: &str,
        scope: &[String],
        name: &hir::ScopedName,
    ) -> IdlcResult<String> {
        let mut name = name.clone();
        if name.name.first().is_some_and(|first| first == version) {
            name.name.remove(0);
            name.is_root = true;
        }
        resolve_canonical(scope, &name, |key| {
            self.versions
                .get(version)
                .is_some_and(|types| types.contains_key(key))
        })
        .ok_or_else(|| Self::error(&name.name.join("::"), "cannot resolve wire reference"))
    }
}
