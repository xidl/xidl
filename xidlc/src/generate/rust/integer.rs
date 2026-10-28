//! Constant-context range checks for IDL integer expressions.

use super::RustRenderer;
use super::util::{rust_literal, rust_scoped_name};
use crate::error::IdlcResult;
use crate::generate::integer_expr::IntegerConstant;
use serde::Serialize;
use xidl_parser::hir;

#[derive(Serialize)]
struct IntegerTemplate {
    name: String,
    ty: String,
    result: String,
    steps: Vec<IntegerStepTemplate>,
}

#[derive(Serialize)]
struct IntegerStepTemplate {
    name: String,
    value: String,
    minimum: String,
    maximum: String,
    check: bool,
    shift: Option<String>,
}

impl RustRenderer {
    pub(super) fn integer_constant(&self, constant: &hir::ConstDcl) -> IdlcResult<Option<String>> {
        let Some(expression) = &constant.integer else {
            return Ok(None);
        };
        let program = IntegerConstant::lower(expression, "_xidl_")?;
        let steps = program
            .steps
            .iter()
            .map(|step| {
                let (minimum, maximum) = step.kind.bounds();
                IntegerStepTemplate {
                    name: step.name.clone(),
                    value: self.integer_expression(&step.value),
                    minimum: minimum.to_string(),
                    maximum: maximum.to_string(),
                    check: step.check,
                    shift: step
                        .shift
                        .as_ref()
                        .map(|value| self.integer_expression(value)),
                }
            })
            .collect();
        let context = IntegerTemplate {
            name: constant.ident.clone(),
            ty: super::util::rust_const_type(&constant.ty),
            result: program.result,
            steps,
        };
        self.render_template("integer.rs.j2", &context).map(Some)
    }

    fn integer_expression(&self, expression: &hir::ConstExpr) -> String {
        crate::generate::render_const_expr(
            expression,
            &rust_scoped_name,
            &|value| match value {
                hir::Literal::IntegerLiteral(value) => format!("{}_i128", value.0),
                _ => rust_literal(value),
            },
            &|operator| match operator {
                hir::UnaryOperator::Add => "",
                hir::UnaryOperator::Sub => "-",
                hir::UnaryOperator::Not => "!",
            },
        )
    }
}
