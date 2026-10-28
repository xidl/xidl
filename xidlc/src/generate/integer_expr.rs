//! Lower typed IDL integer operations to mathematical constant expressions.

use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir::{self, BinaryOperator as Op, ConstExpr as Expr};

/// Constant steps and their target-compiler constraints.
pub(crate) struct IntegerConstant {
    pub(crate) steps: Vec<IntegerStep>,
    pub(crate) result: String,
}

/// One shared expression, referenced by subsequent steps without inlining.
pub(crate) struct IntegerStep {
    pub(crate) name: String,
    pub(crate) value: Expr,
    pub(crate) kind: hir::IntegerKind,
    pub(crate) check: bool,
    pub(crate) shift: Option<Expr>,
}

impl IntegerConstant {
    pub(crate) fn lower(expression: &hir::IntegerExpression, prefix: &str) -> IdlcResult<Self> {
        let mut program = Self {
            steps: Vec::new(),
            result: String::new(),
        };
        for step in &expression.steps {
            let name = format!("{prefix}{}", program.steps.len());
            let (value, check, shift) = program.expression(step)?;
            program.steps.push(IntegerStep {
                name,
                value,
                kind: step.kind,
                check,
                shift,
            });
        }
        program.result = program.step(expression.result)?.name.clone();
        Ok(program)
    }

    fn step(&self, id: hir::IntegerStepId) -> IdlcResult<&IntegerStep> {
        self.steps.get(id.0).ok_or_else(|| {
            IdlcError::template(format!(
                "integer expression step {} does not reference an earlier step",
                id.0
            ))
        })
    }

    fn reference(&self, id: hir::IntegerStepId) -> IdlcResult<Expr> {
        Ok(Expr::ScopedName(hir::ScopedName {
            is_root: false,
            name: vec![self.step(id)?.name.clone()],
        }))
    }

    fn expression(&self, expression: &hir::IntegerStep) -> IdlcResult<(Expr, bool, Option<Expr>)> {
        use hir::IntegerExpressionValue as Value;
        let (value, check, shift) = match &expression.value {
            Value::Literal(value) => (
                Expr::Literal(hir::Literal::IntegerLiteral(hir::IntegerLiteral(
                    value.clone(),
                ))),
                false,
                None,
            ),
            Value::Constant(id) => {
                let (minimum, maximum) = expression.kind.bounds();
                let (inner_minimum, inner_maximum) = self.step(*id)?.kind.bounds();
                (
                    self.reference(*id)?,
                    minimum > inner_minimum || maximum < inner_maximum,
                    None,
                )
            }
            Value::Unary(operator, id) => {
                let value = Expr::UnaryExpr(operator.clone(), Box::new(self.reference(*id)?));
                let value = if matches!(operator, hir::UnaryOperator::Not) {
                    Self::bits(value, expression.kind)
                } else {
                    value
                };
                (value, true, None)
            }
            Value::Binary(operator, left, right) => {
                let left = self.reference(*left)?;
                let right = self.reference(*right)?;
                let shift =
                    matches!(operator, Op::LeftShift | Op::RightShift).then(|| right.clone());
                (
                    Self::binary(*operator, [left, right], expression.kind),
                    true,
                    shift,
                )
            }
        };
        Ok((value, check, shift))
    }

    fn binary(operator: Op, operands: [Expr; 2], kind: hir::IntegerKind) -> Expr {
        let [left, right] = operands;
        match operator {
            // Native shifts may discard high bits before a range check.
            Op::LeftShift => Self::operation(
                Op::Mult,
                left,
                Self::operation(Op::LeftShift, Self::literal(1), right),
            ),
            Op::RightShift => {
                let left = Self::operation(Op::And, left, Self::literal(kind.mask()));
                Self::bits(Self::operation(Op::RightShift, left, right), kind)
            }
            Op::Or | Op::And | Op::Xor => Self::bits(Self::operation(operator, left, right), kind),
            _ => Self::operation(operator, left, right),
        }
    }

    fn bits(value: Expr, kind: hir::IntegerKind) -> Expr {
        let value = Self::operation(Op::And, value, Self::literal(kind.mask()));
        if kind.is_signed() {
            let sign = 1_i128 << (kind.bits() - 1);
            Self::operation(
                Op::Sub,
                Self::operation(Op::Xor, value, Self::literal(sign)),
                Self::literal(sign),
            )
        } else {
            value
        }
    }

    fn operation(operator: Op, left: Expr, right: Expr) -> Expr {
        Expr::BinaryExpr(operator, Box::new(left), Box::new(right))
    }

    fn literal(value: i128) -> Expr {
        Expr::Literal(hir::Literal::IntegerLiteral(hir::IntegerLiteral(
            value.to_string(),
        )))
    }
}
