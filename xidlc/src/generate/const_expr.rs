use xidl_parser::hir;

/// Renders expressions with the names, literals and unary syntax of a target.
pub fn render_const_expr<FScoped, FLit, FUnary>(
    expr: &hir::ConstExpr,
    scoped_name: &FScoped,
    literal: &FLit,
    unary: &FUnary,
) -> String
where
    FScoped: Fn(&hir::ScopedName) -> String,
    FLit: Fn(&hir::Literal) -> String,
    FUnary: Fn(&hir::UnaryOperator) -> &'static str,
{
    match expr {
        hir::ConstExpr::ScopedName(value) => scoped_name(value),
        hir::ConstExpr::Literal(value) => literal(value),
        hir::ConstExpr::UnaryExpr(op, value) => {
            let op = unary(op);
            format!(
                "({op}{})",
                render_const_expr(value, scoped_name, literal, unary)
            )
        }
        hir::ConstExpr::BinaryExpr(op, left, right) => {
            let op = match op {
                hir::BinaryOperator::Or => "|",
                hir::BinaryOperator::Xor => "^",
                hir::BinaryOperator::And => "&",
                hir::BinaryOperator::LeftShift => "<<",
                hir::BinaryOperator::RightShift => ">>",
                hir::BinaryOperator::Add => "+",
                hir::BinaryOperator::Sub => "-",
                hir::BinaryOperator::Mult => "*",
                hir::BinaryOperator::Div => "/",
                hir::BinaryOperator::Mod => "%",
            };
            format!(
                "({} {op} {})",
                render_const_expr(left, scoped_name, literal, unary),
                render_const_expr(right, scoped_name, literal, unary)
            )
        }
    }
}
