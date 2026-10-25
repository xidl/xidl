//! Resolve model names with the ordinary generator's identifier and type rules.

use super::context::{Call, Value};
use super::layout::Layout;
use crate::error::IdlcResult;
use crate::generate::adapter::{TypeShape, TypeUse, ValuePlan};
use crate::generate::rust::util::{rust_ident, type_with_decl};
use crate::generate::utils::scope::resolve_canonical;
use convert_case::{Case, Casing};
use std::collections::BTreeMap;
use xidl_parser::hir;

pub(super) struct Names<'a> {
    pub version: &'a str,
    pub step_from: &'a str,
    pub upward: bool,
    pub types: &'a BTreeMap<String, TypeShape>,
    pub layout: &'a Layout<'a>,
}

impl Names<'_> {
    pub(super) fn named(&self, key: &str) -> String {
        std::iter::once(self.version)
            .chain(key.split("::"))
            .map(rust_ident)
            .collect::<Vec<_>>()
            .join("::")
    }

    pub(super) fn function(&self, key: &str) -> String {
        let direction = if self.upward {
            "upcast_from"
        } else {
            "downcast_to"
        };
        format!(
            "{}_{}_{}",
            key.replace("::", "_").to_case(Case::Snake),
            direction,
            self.step_from.to_case(Case::Snake)
        )
    }

    pub(super) fn decision(&self, key: &str) -> String {
        format!("{}Decision", self.function(key).to_case(Case::Pascal))
    }

    pub(super) fn hook(&self, key: &str) -> String {
        self.function(key)
    }

    /// Stored-value accessor for one removed member of a type or response.
    pub(super) fn reader(key: &str, member: &str) -> String {
        format!("{}_{member}", key.replace("::", "_")).to_case(Case::Snake)
    }

    /// Persisted-member bag of one update operation, named by the operation.
    pub(crate) fn stored_type(key: &str) -> String {
        format!(
            "{}Stored",
            key.replace("::", "_")
                .replace('.', "_")
                .to_case(Case::Pascal)
        )
    }

    pub(super) fn value_type(&self, value: &TypeUse, recursive: bool) -> IdlcResult<String> {
        let mut ty = value.ty.clone();
        self.normalize(&mut ty, &value.scope)?;
        let mut declarator = value.declarator.clone();
        if let hir::Declarator::ArrayDeclarator(array) = &mut declarator {
            for dimension in &mut array.len {
                self.normalize_constant(&mut dimension.0, &value.scope)?;
            }
        }
        let mut ty = type_with_decl(&ty, &declarator);
        if recursive {
            ty = format!("Box<{ty}>");
        }
        if value.optional {
            ty = format!("Option<{ty}>");
        }
        Ok(ty)
    }

    pub(super) fn type_name(&self, ty: &hir::TypeSpec, scope: &[String]) -> IdlcResult<String> {
        let mut ty = ty.clone();
        self.normalize(&mut ty, scope)?;
        Ok(crate::generate::rust::util::rust_type(&ty))
    }

    pub(super) fn value(&self, plan: &ValuePlan) -> Value {
        match plan {
            ValuePlan::Identity => Value::Identity,
            ValuePlan::Named(key) => Value::Named(Call {
                name: self.function(key),
                stored: false,
                current: false,
                pair: false,
            }),
            ValuePlan::Sequence(value) => Value::Sequence(Box::new(self.value(value))),
            ValuePlan::Map { key, value } => Value::Map {
                key: Box::new(self.value(key)),
                value: Box::new(self.value(value)),
            },
            ValuePlan::Optional(value) => Value::Optional(Box::new(self.value(value))),
            ValuePlan::Array(value) => Value::Array(Box::new(self.value(value))),
        }
    }

    pub(super) fn field_value(&self, plan: &ValuePlan, boxed: (bool, bool)) -> Value {
        if !boxed.0 && !boxed.1 {
            return self.value(plan);
        }
        let wrap = |plan| Value::Boxed {
            source: boxed.0,
            target: boxed.1,
            value: Box::new(self.value(plan)),
        };
        match plan {
            ValuePlan::Optional(value) => Value::Optional(Box::new(wrap(value))),
            _ => wrap(plan),
        }
    }

    fn normalize(&self, ty: &mut hir::TypeSpec, scope: &[String]) -> IdlcResult<()> {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                let key = self.resolve(name, scope)?;
                name.name = std::iter::once(self.version.to_string())
                    .chain(key.split("::").map(str::to_string))
                    .collect();
                name.is_root = false;
            }
            hir::TypeSpec::SequenceType(value) => self.normalize(&mut value.ty, scope)?,
            hir::TypeSpec::MapType(value) => {
                self.normalize(&mut value.key, scope)?;
                self.normalize(&mut value.value, scope)?;
            }
            hir::TypeSpec::TemplateType(value) => {
                return Err(Layout::error(
                    &value.ident,
                    "custom templates have no declared Rust adapter semantics",
                ));
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn resolve(&self, name: &hir::ScopedName, scope: &[String]) -> IdlcResult<String> {
        resolve_canonical(scope, name, |key| self.types.contains_key(key))
            .ok_or_else(|| Layout::error(&name.name.join("::"), "cannot resolve public model type"))
    }

    fn normalize_constant(
        &self,
        expression: &mut hir::ConstExpr,
        scope: &[String],
    ) -> IdlcResult<()> {
        match expression {
            hir::ConstExpr::ScopedName(name) => {
                let scope = std::iter::once(self.version.to_string())
                    .chain(scope.iter().cloned())
                    .collect::<Vec<_>>();
                if name.is_root {
                    name.name.insert(0, self.version.to_string());
                }
                let key =
                    resolve_canonical(&scope, name, |key| self.layout.constants.contains(key))
                        .ok_or_else(|| {
                            Layout::error(
                                &name.name.join("::"),
                                "cannot resolve public array bound",
                            )
                        })?;
                name.name = key.split("::").map(str::to_string).collect();
                name.is_root = false;
            }
            hir::ConstExpr::UnaryExpr(_, value) => self.normalize_constant(value, scope)?,
            hir::ConstExpr::BinaryExpr(_, left, right) => {
                self.normalize_constant(left, scope)?;
                self.normalize_constant(right, scope)?;
            }
            hir::ConstExpr::Literal(_) => {}
        }
        Ok(())
    }

    pub(super) fn bound(
        &self,
        bound: &hir::PositiveIntConst,
        scope: &[String],
    ) -> IdlcResult<String> {
        let mut value = bound.0.clone();
        self.normalize_constant(&mut value, scope)?;
        Ok(crate::generate::rust::util::render_const(&value))
    }
}
