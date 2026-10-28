//! Bind ordinary integer expressions as a graph without computing their values.

use super::imports::IntegerScope;
use super::integer::{Integer, IntegerKind};
use super::resolver::IntegerConstants;
use crate::error::{ParseError, ParserResult};
use crate::hir::{self, IntegerExpression, IntegerExpressionValue as Value, IntegerStepId};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Operand {
    id: IntegerStepId,
    kind: IntegerKind,
}

pub(super) struct IntegerExpressions<'a> {
    constants: &'a mut IntegerConstants,
    steps: Vec<hir::IntegerStep>,
    references: BTreeMap<IntegerScope, Operand>,
}

impl<'a> IntegerExpressions<'a> {
    pub(super) fn resolve(
        constants: &'a mut IntegerConstants,
        constant: &hir::ConstDcl,
        scope: &IntegerScope,
    ) -> ParserResult<IntegerExpression> {
        let mut expressions = Self {
            constants,
            steps: Vec::new(),
            references: BTreeMap::new(),
        };
        let result = expressions.constant(constant, scope)?;
        Ok(IntegerExpression {
            steps: expressions.steps,
            result: result.id,
        })
    }

    fn push(&mut self, kind: IntegerKind, value: Value) -> Operand {
        let id = IntegerStepId(self.steps.len());
        self.steps.push(hir::IntegerStep { kind, value });
        Operand { id, kind }
    }

    fn boundary(&mut self, kind: IntegerKind, inner: Operand) -> Operand {
        if kind == inner.kind {
            inner
        } else {
            self.push(kind, Value::Constant(inner.id))
        }
    }

    fn constant(
        &mut self,
        constant: &hir::ConstDcl,
        scope: &IntegerScope,
    ) -> ParserResult<Operand> {
        let kind = self.constants.kind(&constant.ty, scope)?.ok_or_else(|| {
            ParseError::Message(format!(
                "integer constant '{}' is not integral",
                constant.ident
            ))
        })?;
        let expression = self.expression(&constant.value, scope, Some(kind.precision()))?;
        Ok(self.boundary(kind, expression))
    }

    fn expression(
        &mut self,
        expression: &hir::ConstExpr,
        scope: &IntegerScope,
        precision: Option<u32>,
    ) -> ParserResult<Operand> {
        match expression {
            hir::ConstExpr::ScopedName(name) => {
                let expression = self.reference(name, scope)?;
                Ok(self.boundary(expression.kind.with_precision(precision), expression))
            }
            hir::ConstExpr::Literal(hir::Literal::IntegerLiteral(value)) => {
                let kind = Integer::literal(&value.0, precision)?.kind();
                Ok(self.push(kind, Value::Literal(value.0.clone())))
            }
            hir::ConstExpr::Literal(_) => Err(ParseError::Message(
                "integer constant expression is not an integer".into(),
            )),
            hir::ConstExpr::UnaryExpr(operator, inner) => {
                let inner = self.expression(inner, scope, precision)?;
                let kind = if matches!(operator, hir::UnaryOperator::Sub) {
                    IntegerKind {
                        signed: true,
                        ..inner.kind.promoted()
                    }
                } else {
                    inner.kind.promoted()
                };
                Ok(self.push(kind, Value::Unary(operator.clone(), inner.id)))
            }
            hir::ConstExpr::BinaryExpr(operator, left, right) => {
                let left = self.expression(left, scope, precision)?;
                let right = self.expression(right, scope, precision)?;
                Ok(self.push(
                    left.kind.common(right.kind),
                    Value::Binary(*operator, left.id, right.id),
                ))
            }
        }
    }

    fn reference(&mut self, name: &hir::ScopedName, scope: &IntegerScope) -> ParserResult<Operand> {
        let resolved = self.constants.resolve(name, scope)?;
        if let Some(expression) = self.references.get(&resolved) {
            return Ok(*expression);
        }
        let key = resolved.path.join("::");
        let constant = self
            .constants
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
        self.constants.enter(&resolved)?;
        let result = self.constant(&constant, &resolved.parent());
        self.constants.active.remove(&resolved);
        if let Ok(expression) = &result {
            self.references.insert(resolved.clone(), *expression);
        }
        result.map_err(|error| {
            ParseError::Message(format!("constant '{}': {error}", resolved.qualified()))
        })
    }
}
