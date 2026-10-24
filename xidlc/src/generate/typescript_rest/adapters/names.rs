//! Reuse the regular generator's identifiers and type rendering rules.

use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::{Member, TypeUse, ValuePlan, ValueRepresentation};
use crate::generate::typescript::definition::TypeRefTarget;
use crate::generate::typescript::definition::contexts::TsType;
use crate::generate::typescript::definition::names::{method_struct_prefix, scoped_name};
use crate::generate::typescript::definition::type_expr::ts_type_for_decl;
use crate::generate::utils::scope::resolve_canonical;
use convert_case::{Case, Casing};
use std::collections::BTreeSet;
use xidl_parser::hir;

pub(super) struct Names<'a> {
    pub keys: &'a BTreeSet<String>,
    pub version: &'a str,
    pub from: &'a str,
    pub upward: bool,
}

impl Names<'_> {
    pub fn function(&self, key: &str) -> String {
        let name = key.replace("::", "_").to_case(Case::Pascal);
        let from = self.from.to_case(Case::Pascal);
        if self.upward {
            format!("upcast{name}From{from}")
        } else {
            format!("downcast{name}To{from}")
        }
    }

    pub fn hook(&self, key: &str) -> String {
        let name = key.replace("::", "_").to_case(Case::Camel);
        let from = self.from.to_case(Case::Pascal);
        if self.upward {
            format!("{name}UpcastFrom{from}")
        } else {
            format!("{name}DowncastTo{from}")
        }
    }

    pub fn decision(&self, key: &str) -> String {
        let name = key.replace("::", "_").to_case(Case::Pascal);
        let from = self.from.to_case(Case::Pascal);
        if self.upward {
            format!("{name}UpcastDecisionFrom{from}")
        } else {
            format!("{name}DowncastDecisionTo{from}")
        }
    }

    /// Stored-value accessor for one removed member of a type or response.
    pub fn reader(key: &str, member: &str) -> String {
        format!("{}_{member}", key.replace("::", "_")).to_case(Case::Camel)
    }

    /// Persisted-member bag of one update operation, named by the operation.
    pub fn stored_type(key: &str) -> String {
        format!(
            "{}Stored",
            key.replace("::", "_")
                .replace('.', "_")
                .to_case(Case::Pascal)
        )
    }

    pub fn named(&self, key: &str) -> TsType {
        let path = std::iter::once(self.version.to_string())
            .chain(key.split("::").map(str::to_string))
            .collect::<Vec<_>>();
        TsType::ScopedName(format!(
            "types.{}",
            scoped_name(&path[..path.len() - 1], &path[path.len() - 1])
        ))
    }

    pub fn operation_type(&self, interface: &str, operation: &str, suffix: &str) -> TsType {
        let parts = interface.split("::").collect::<Vec<_>>();
        let prefix = method_struct_prefix(parts.last().copied().unwrap_or(interface), operation);
        let path = std::iter::once(self.version.to_string())
            .chain(
                parts[..parts.len().saturating_sub(1)]
                    .iter()
                    .map(|part| part.to_string()),
            )
            .collect::<Vec<_>>();
        TsType::ScopedName(format!(
            "ifaceTypes.{}",
            scoped_name(&path, &format!("{prefix}{suffix}"))
        ))
    }

    pub fn member_type(&self, member: &Member) -> IdlcResult<TsType> {
        let value = member.value.as_ref().ok_or_else(|| {
            IdlcError::rpc(format!(
                "typescript adapters: member {} has no type",
                member.name
            ))
        })?;
        if value.representation == ValueRepresentation::ByteBuffer {
            return Ok(TsType::Primitive("Uint8Array".into()));
        }
        let mut ty = value.ty.clone();
        self.qualify(&mut ty, &value.scope)?;
        Ok(ts_type_for_decl(
            &ty,
            &value.declarator,
            &[],
            TypeRefTarget::Client,
        ))
    }

    pub fn value(&self, value: &ValuePlan) -> IdlcResult<super::context::Value> {
        use super::context::{Call, Value};
        Ok(match value {
            ValuePlan::Identity => Value::Identity,
            ValuePlan::Named(key) => Value::Named(Call {
                name: self.function(key),
                stored: false,
                current: false,
                pair: false,
            }),
            ValuePlan::Sequence(value) | ValuePlan::Array(value) => {
                Value::Sequence(Box::new(self.value(value)?))
            }
            ValuePlan::Optional(value) => Value::Optional(Box::new(self.value(value)?)),
            ValuePlan::Map { key, value } => {
                if !matches!(key.as_ref(), ValuePlan::Identity) {
                    return Err(IdlcError::rpc(
                        "typescript adapters: named map keys require a wire-key conversion policy",
                    ));
                }
                Value::Map(Box::new(self.value(value)?))
            }
        })
    }

    pub fn optional(value: &TypeUse) -> bool {
        value.optional
    }

    fn qualify(&self, ty: &mut hir::TypeSpec, scope: &[String]) -> IdlcResult<()> {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                if name.name.first().is_some_and(|first| first == self.version) {
                    name.name.remove(0);
                    name.is_root = true;
                }
                let key = resolve_canonical(scope, name, |key| self.keys.contains(key))
                    .ok_or_else(|| {
                        IdlcError::rpc(format!(
                            "typescript adapters: cannot resolve type {} in {}",
                            name.name.join("::"),
                            scope.join("::")
                        ))
                    })?;
                name.name = std::iter::once(self.version.to_string())
                    .chain(key.split("::").map(str::to_string))
                    .collect();
                name.is_root = true;
            }
            hir::TypeSpec::SequenceType(sequence) => self.qualify(&mut sequence.ty, scope)?,
            hir::TypeSpec::MapType(map) => {
                self.qualify(&mut map.key, scope)?;
                self.qualify(&mut map.value, scope)?;
            }
            hir::TypeSpec::TemplateType(template) => {
                for arg in &mut template.args {
                    self.qualify(arg, scope)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
