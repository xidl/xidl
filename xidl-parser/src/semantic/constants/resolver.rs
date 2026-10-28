//! Scoped integer constants shared by declarations and fixed-array dimensions.

use crate::error::{ParseError, ParserResult};
use crate::hir;
use std::collections::{BTreeMap, BTreeSet};

use super::declarations::IntegerIndex;
use super::imports::{IntegerImports, IntegerOrigin, IntegerScope};
use super::integer::{Integer, IntegerKind};

/// Resolves integer expressions in the unit and version that declared them.
pub(in crate::semantic) struct IntegerConstants {
    local: IntegerIndex,
    imports: IntegerImports,
    values: BTreeMap<IntegerScope, Integer>,
    pub(super) active: BTreeSet<IntegerScope>,
}

impl IntegerConstants {
    /// Adds immutable imported source declarations to the shared resolver.
    pub(in crate::semantic) fn with_imports(
        spec: &hir::Specification,
        imports: &IntegerImports,
    ) -> Self {
        Self {
            local: IntegerIndex::collect(spec),
            imports: imports.clone(),
            values: BTreeMap::new(),
            active: BTreeSet::new(),
        }
    }

    pub(super) fn index(&self, scope: &IntegerScope) -> ParserResult<&IntegerIndex> {
        match &scope.origin {
            IntegerOrigin::Local => Ok(&self.local),
            _ => self.imports.index(&scope.origin).ok_or_else(|| {
                ParseError::Message(format!(
                    "integer declaration context '{}' does not resolve",
                    scope.qualified()
                ))
            }),
        }
    }

    fn binding(
        &self,
        name: &hir::ScopedName,
        scope: &IntegerScope,
    ) -> ParserResult<Option<IntegerScope>> {
        let index = self.index(scope)?;
        if let Some(resolved) = index.names.resolve(&scope.path, name) {
            return Ok(Some(IntegerScope {
                origin: scope.origin.clone(),
                path: resolved.name,
            }));
        }
        Ok(self.imports.resolve(&scope.origin, name))
    }

    /// Evaluates a dimension using IDL integer precision and checked operations.
    pub(in crate::semantic) fn dimension(
        &mut self,
        expression: &hir::ConstExpr,
        scope: &[String],
    ) -> ParserResult<i64> {
        self.evaluate(expression, &IntegerScope::local(scope), None)?
            .dimension()
    }

    /// Resolves integer types and references while retaining the source expression.
    pub(super) fn normalize_declaration(
        &mut self,
        constant: &mut hir::ConstDcl,
        scope: &[String],
    ) -> ParserResult<()> {
        if constant.integer.is_some() {
            return Ok(());
        }
        let scope = IntegerScope::local(scope);
        if self.kind(&constant.ty, &scope)?.is_none()
            || !self.is_known(&constant.value, &scope, &mut BTreeSet::new())?
        {
            return Ok(());
        }
        constant.integer = Some(
            super::expressions::IntegerExpressions::resolve(self, constant, &scope).map_err(
                |error| {
                    let mut declaration = scope.clone();
                    declaration.path.push(constant.ident.clone());
                    ParseError::Message(format!("constant '{}': {error}", declaration.qualified()))
                },
            )?,
        );
        Ok(())
    }

    fn is_known(
        &mut self,
        expression: &hir::ConstExpr,
        scope: &IntegerScope,
        visited: &mut BTreeSet<IntegerScope>,
    ) -> ParserResult<bool> {
        match expression {
            hir::ConstExpr::ScopedName(name) => {
                let Some(resolved) = self.binding(name, scope)? else {
                    return Ok(false);
                };
                let key = resolved.path.join("::");
                let Some(constant) = self.index(&resolved)?.declarations.get(&key).cloned() else {
                    return Ok(false);
                };
                if !visited.insert(resolved.clone()) {
                    return Ok(true);
                }
                let parent = resolved.parent();
                Ok(self.kind(&constant.ty, &parent)?.is_some()
                    && self.is_known(&constant.value, &parent, visited)?)
            }
            hir::ConstExpr::UnaryExpr(_, inner) => self.is_known(inner, scope, visited),
            hir::ConstExpr::BinaryExpr(_, left, right) => {
                Ok(self.is_known(left, scope, visited)? && self.is_known(right, scope, visited)?)
            }
            hir::ConstExpr::Literal(_) => Ok(true),
        }
    }

    fn evaluate(
        &mut self,
        expression: &hir::ConstExpr,
        scope: &IntegerScope,
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
                "integer constant expression is not an integer".into(),
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

    pub(super) fn resolve(
        &self,
        name: &hir::ScopedName,
        scope: &IntegerScope,
    ) -> ParserResult<IntegerScope> {
        self.binding(name, scope)?.ok_or_else(|| {
            ParseError::Message(format!(
                "integer constant reference '{}' does not resolve",
                name.name.join("::")
            ))
        })
    }

    pub(super) fn enter(&mut self, key: &IntegerScope) -> ParserResult<()> {
        if self.active.insert(key.clone()) {
            Ok(())
        } else {
            Err(ParseError::Message(format!(
                "integer constant/type cycle includes '{}'",
                key.qualified()
            )))
        }
    }

    fn reference(&mut self, name: &hir::ScopedName, scope: &IntegerScope) -> ParserResult<Integer> {
        let resolved = self.resolve(name, scope)?;
        if let Some(value) = self.values.get(&resolved) {
            return Ok(*value);
        }
        let key = resolved.path.join("::");
        let constant = self
            .index(&resolved)?
            .declarations
            .get(&key)
            .cloned()
            .ok_or_else(|| {
                ParseError::Message(format!(
                    "integer constant reference '{}' is not a constant",
                    resolved.qualified()
                ))
            })?;
        self.enter(&resolved)?;
        let result = self.constant(&constant, &resolved.parent());
        self.active.remove(&resolved);
        if let Ok(value) = result {
            self.values.insert(resolved.clone(), value);
        }
        result.map_err(|error| {
            ParseError::Message(format!("constant '{}': {error}", resolved.qualified()))
        })
    }

    fn constant(
        &mut self,
        constant: &hir::ConstDcl,
        scope: &IntegerScope,
    ) -> ParserResult<Integer> {
        let kind = self.kind(&constant.ty, scope)?.ok_or_else(|| {
            ParseError::Message(format!(
                "integer constant '{}' is not integral",
                constant.ident
            ))
        })?;
        self.evaluate(&constant.value, scope, Some(kind.precision()))?
            .assign(kind)
    }

    pub(super) fn kind(
        &mut self,
        ty: &hir::ConstType,
        scope: &IntegerScope,
    ) -> ParserResult<Option<IntegerKind>> {
        match ty {
            hir::ConstType::IntegerType(kind) => Ok(Some(IntegerKind::from_hir(kind))),
            hir::ConstType::OctetType => Ok(Some(IntegerKind::from_hir(&hir::IntegerType::Octet))),
            hir::ConstType::ScopedName(name) => self.named_kind(name, scope),
            _ => Ok(None),
        }
    }

    fn named_kind(
        &mut self,
        name: &hir::ScopedName,
        scope: &IntegerScope,
    ) -> ParserResult<Option<IntegerKind>> {
        let Some(resolved) = self.binding(name, scope)? else {
            return Ok(None);
        };
        let key = resolved.path.join("::");
        let Some(ty) = self.index(&resolved)?.aliases.get(&key).cloned() else {
            return Ok(None);
        };
        self.enter(&resolved)?;
        let result = match &ty {
            hir::TypeSpec::IntegerType(kind) => Ok(Some(IntegerKind::from_hir(kind))),
            hir::TypeSpec::ScopedName(name) => self.named_kind(name, &resolved.parent()),
            _ => Ok(None),
        };
        self.active.remove(&resolved);
        result
    }
}
