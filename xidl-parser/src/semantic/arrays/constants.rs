//! Integer expressions used by fixed-array declarations.

use crate::error::{ParseError, ParserResult};
use crate::hir::{self, DeclarationNames};
use std::collections::{BTreeMap, BTreeSet};

use super::integer::{Integer, IntegerKind};

/// Resolves constant expressions in their declaration's lexical scope.
pub(super) struct IntegerConstants {
    names: DeclarationNames,
    declarations: BTreeMap<String, hir::ConstDcl>,
    aliases: BTreeMap<String, hir::TypeSpec>,
    values: BTreeMap<String, Integer>,
    active: BTreeSet<String>,
}

impl IntegerConstants {
    /// Collects declarations without evaluating unrelated constant values.
    pub(super) fn collect(spec: &hir::Specification) -> Self {
        let mut constants = Self {
            names: DeclarationNames::collect(&spec.0),
            declarations: BTreeMap::new(),
            aliases: BTreeMap::new(),
            values: BTreeMap::new(),
            active: BTreeSet::new(),
        };
        constants.definitions(&spec.0, &[]);
        constants
    }

    fn definitions(&mut self, definitions: &[hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.definitions(&module.definition, &nested);
                }
                hir::Definition::ConstDcl(constant) => self.insert(constant, scope),
                hir::Definition::TypeDcl(decl) => self.alias(decl, scope),
                hir::Definition::InterfaceDcl(interface) => {
                    if let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl
                        && let Some(body) = &def.interface_body
                    {
                        let mut nested = scope.to_vec();
                        nested.push(def.header.ident.clone());
                        for export in &body.0 {
                            match export {
                                hir::Export::ConstDcl(constant) => self.insert(constant, &nested),
                                hir::Export::TypeDcl(decl) => self.alias(decl, &nested),
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn key(scope: &[String], name: &str) -> String {
        scope
            .iter()
            .map(String::as_str)
            .chain([name])
            .collect::<Vec<_>>()
            .join("::")
    }

    fn insert(&mut self, constant: &hir::ConstDcl, scope: &[String]) {
        self.declarations
            .insert(Self::key(scope, &constant.ident), constant.clone());
    }

    fn alias(&mut self, decl: &hir::TypeDcl, scope: &[String]) {
        if let hir::TypeDcl::TypedefDcl(alias) = decl
            && let hir::TypedefType::TypeSpec(ty) = &alias.ty
        {
            for declarator in &alias.decl {
                if let hir::Declarator::SimpleDeclarator(name) = declarator {
                    self.aliases.insert(Self::key(scope, &name.0), ty.clone());
                }
            }
        }
    }

    /// Evaluates a dimension using IDL integer precision and checked operations.
    pub(super) fn dimension(
        &mut self,
        expression: &hir::ConstExpr,
        scope: &[String],
    ) -> ParserResult<i64> {
        self.evaluate(expression, scope, None)?.dimension()
    }

    fn evaluate(
        &mut self,
        expression: &hir::ConstExpr,
        scope: &[String],
        precision: Option<u32>,
    ) -> ParserResult<Integer> {
        match expression {
            hir::ConstExpr::ScopedName(name) => {
                self.reference(name, scope)?.with_precision(precision)
            }
            hir::ConstExpr::Literal(hir::Literal::IntegerLiteral(value)) => {
                Integer::literal(&value.0, precision)
            }
            hir::ConstExpr::Literal(_) => Err(ParseError::Message(
                "array size expression is not an integer".into(),
            )),
            hir::ConstExpr::UnaryExpr(operator, inner) => {
                self.evaluate(inner, scope, precision)?.unary(operator)
            }
            hir::ConstExpr::BinaryExpr(operator, left, right) => {
                let left = self.evaluate(left, scope, precision)?;
                let right = self.evaluate(right, scope, precision)?;
                left.binary(*operator, right)
            }
        }
    }

    fn resolve(&self, name: &hir::ScopedName, scope: &[String]) -> ParserResult<hir::ScopedName> {
        self.names.resolve(scope, name).ok_or_else(|| {
            ParseError::Message(format!(
                "array size reference '{}' does not resolve",
                name.name.join("::")
            ))
        })
    }

    fn enter(&mut self, key: &str) -> ParserResult<()> {
        if self.active.insert(key.to_owned()) {
            Ok(())
        } else {
            Err(ParseError::Message(format!(
                "array size constant/type cycle includes '{key}'"
            )))
        }
    }

    fn reference(&mut self, name: &hir::ScopedName, scope: &[String]) -> ParserResult<Integer> {
        let resolved = self.resolve(name, scope)?;
        let key = resolved.name.join("::");
        if let Some(value) = self.values.get(&key) {
            return Ok(*value);
        }
        let constant = self.declarations.get(&key).cloned().ok_or_else(|| {
            ParseError::Message(format!("array size reference '{key}' is not a constant"))
        })?;
        self.enter(&key)?;
        let parent = &resolved.name[..resolved.name.len() - 1];
        let result = self.constant(&constant, parent);
        self.active.remove(&key);
        if let Ok(value) = result {
            self.values.insert(key.clone(), value);
        }
        result.map_err(|error| ParseError::Message(format!("constant '{key}': {error}")))
    }

    fn constant(&mut self, constant: &hir::ConstDcl, scope: &[String]) -> ParserResult<Integer> {
        let kind = match &constant.ty {
            hir::ConstType::IntegerType(kind) => IntegerKind::from_hir(kind),
            hir::ConstType::OctetType => IntegerKind::from_hir(&hir::IntegerType::Octet),
            hir::ConstType::ScopedName(name) => self.named_kind(name, scope)?,
            _ => {
                return Err(ParseError::Message(
                    "array size constant is not integral".into(),
                ));
            }
        };
        self.evaluate(&constant.value, scope, Some(kind.precision()))?
            .assign(kind)
    }

    fn named_kind(
        &mut self,
        name: &hir::ScopedName,
        scope: &[String],
    ) -> ParserResult<IntegerKind> {
        let resolved = self.resolve(name, scope)?;
        let key = resolved.name.join("::");
        let ty = self.aliases.get(&key).cloned().ok_or_else(|| {
            ParseError::Message(format!(
                "array size constant type '{key}' is not an integral alias"
            ))
        })?;
        self.enter(&key)?;
        let result = match &ty {
            hir::TypeSpec::IntegerType(kind) => Ok(IntegerKind::from_hir(kind)),
            hir::TypeSpec::ScopedName(name) => {
                self.named_kind(name, &resolved.name[..resolved.name.len() - 1])
            }
            _ => Err(ParseError::Message(format!(
                "array size constant type '{key}' is not integral"
            ))),
        };
        self.active.remove(&key);
        result
    }
}
