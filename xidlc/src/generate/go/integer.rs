//! Typed range constraints around private untyped Go constant expressions.

use super::GoRenderContext;
use super::definition::go_literal;
use crate::error::IdlcResult;
use crate::generate::integer_expr::IntegerConstant;
use serde::Serialize;
use xidl_parser::hir;

#[derive(Serialize)]
struct IntegerStepTemplate {
    name: String,
    value: String,
    ty: String,
    check: bool,
    shift: Option<String>,
}

impl GoRenderContext {
    pub(crate) fn integer_constant(
        &mut self,
        constant: &hir::ConstDcl,
        name: &str,
    ) -> IdlcResult<Option<String>> {
        let Some(expression) = &constant.integer else {
            return Ok(None);
        };
        let prefix = format!("_xidl_{name}_");
        let program = IntegerConstant::lower(expression, &prefix)?;
        let mut steps = Vec::new();
        for step in &program.steps {
            steps.push(IntegerStepTemplate {
                name: step.name.clone(),
                value: self.integer_expression(&step.value),
                ty: format!(
                    "{}int{}",
                    if step.kind.is_signed() { "" } else { "u" },
                    step.kind.bits()
                ),
                check: step.check,
                shift: step
                    .shift
                    .as_ref()
                    .map(|value| self.integer_expression(value)),
            });
        }
        self.push_template(
            "integer_checks.go.j2",
            &serde_json::json!({ "steps": steps }),
        )?;
        Ok(Some(program.result))
    }
    fn integer_expression(&self, expression: &hir::ConstExpr) -> String {
        crate::generate::render_const_expr(
            expression,
            &|value| value.name.join("."),
            &go_literal,
            &|operator| match operator {
                hir::UnaryOperator::Add => "+",
                hir::UnaryOperator::Sub => "-",
                hir::UnaryOperator::Not => "^",
            },
        )
    }
}
