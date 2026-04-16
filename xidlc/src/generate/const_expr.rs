use xidl_parser::hir;

pub fn render_const_expr<FScoped, FLit>(
    expr: &hir::ConstExpr,
    scoped_name: &FScoped,
    literal: &FLit,
) -> String
where
    FScoped: Fn(&hir::ScopedName) -> String,
    FLit: Fn(&hir::Literal) -> String,
{
    match expr {
        hir::ConstExpr::ScopedName(value) => scoped_name(value),
        hir::ConstExpr::Literal(value) => literal(value),
        hir::ConstExpr::UnaryExpr(op, value) => {
            let op = match op {
                hir::UnaryOperator::Add => "+",
                hir::UnaryOperator::Sub => "-",
                hir::UnaryOperator::Not => "~",
            };
            format!("({op}{})", render_const_expr(value, scoped_name, literal))
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
                render_const_expr(left, scoped_name, literal),
                render_const_expr(right, scoped_name, literal)
            )
        }
    }
}
