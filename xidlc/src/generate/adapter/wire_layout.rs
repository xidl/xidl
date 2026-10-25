//! Assign promoted wire keys to flattened fields.

use super::model::{TypeShape, TypeUse};
use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::scope::resolve_canonical;
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::hir;

#[derive(Default)]
pub(crate) struct WireLayout {
    pub keys: BTreeSet<String>,
    pub catch_all: bool,
}

pub(crate) struct WireLayouts<'a> {
    pub version: &'a str,
    pub types: &'a BTreeMap<String, TypeShape>,
}

impl WireLayouts<'_> {
    pub(crate) fn layout(
        &self,
        key: &str,
        visiting: &mut BTreeSet<String>,
    ) -> IdlcResult<WireLayout> {
        if !visiting.insert(key.to_string()) {
            return Err(Self::error(key, "flattened reference cycle"));
        }
        let shape = self
            .types
            .get(key)
            .ok_or_else(|| Self::error(key, "flattened type is missing"))?;
        let layout = match shape {
            TypeShape::Struct(members) => {
                let mut layout = WireLayout::default();
                for member in members {
                    let next = if member.flatten {
                        self.value_layout(
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
            TypeShape::Alias(value) => self.value_layout(value, visiting)?,
            _ => return Err(Self::error(key, "flatten requires a struct or map")),
        };
        visiting.remove(key);
        Ok(layout)
    }

    pub(crate) fn value_layout(
        &self,
        value: &TypeUse,
        visiting: &mut BTreeSet<String>,
    ) -> IdlcResult<WireLayout> {
        if matches!(value.declarator, hir::Declarator::ArrayDeclarator(_)) {
            return Err(Self::error(self.version, "flatten does not support arrays"));
        }
        match &value.ty {
            hir::TypeSpec::MapType(_)
            | hir::TypeSpec::AnyType
            | hir::TypeSpec::ObjectType
            | hir::TypeSpec::ValueBaseType => Ok(WireLayout {
                keys: BTreeSet::new(),
                catch_all: true,
            }),
            hir::TypeSpec::ScopedName(name) => {
                self.layout(&self.resolve(&value.scope, name)?, visiting)
            }
            _ => Err(Self::error(
                self.version,
                "flatten requires a struct or map",
            )),
        }
    }

    pub(crate) fn resolve(&self, scope: &[String], name: &hir::ScopedName) -> IdlcResult<String> {
        resolve_canonical(scope, name, |key| self.types.contains_key(key))
            .ok_or_else(|| Self::error(&name.name.join("::"), "cannot resolve wire reference"))
    }
    fn error(key: &str, reason: &str) -> IdlcError {
        IdlcError::rpc(format!("version adapter wire layout {key}: {reason}"))
    }
}
