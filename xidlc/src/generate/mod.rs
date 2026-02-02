pub mod c;
pub mod cpp;
pub mod hir_gen;
pub mod rust;
pub mod rust_jsonrpc;
mod utils;

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
    fn render_or<FScoped, FLit>(expr: &hir::OrExpr, scoped_name: &FScoped, literal: &FLit) -> String
    where
        FScoped: Fn(&hir::ScopedName) -> String,
        FLit: Fn(&hir::Literal) -> String,
    {
        match expr {
            hir::OrExpr::XorExpr(value) => render_xor(value, scoped_name, literal),
            hir::OrExpr::OrExpr(left, right) => format!(
                "({} | {})",
                render_or(left, scoped_name, literal),
                render_xor(right, scoped_name, literal)
            ),
        }
    }

    fn render_xor<FScoped, FLit>(
        expr: &hir::XorExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&hir::ScopedName) -> String,
        FLit: Fn(&hir::Literal) -> String,
    {
        match expr {
            hir::XorExpr::AndExpr(value) => render_and(value, scoped_name, literal),
            hir::XorExpr::XorExpr(left, right) => format!(
                "({} ^ {})",
                render_xor(left, scoped_name, literal),
                render_and(right, scoped_name, literal)
            ),
        }
    }

    fn render_and<FScoped, FLit>(
        expr: &hir::AndExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&hir::ScopedName) -> String,
        FLit: Fn(&hir::Literal) -> String,
    {
        match expr {
            hir::AndExpr::ShiftExpr(value) => render_shift(value, scoped_name, literal),
            hir::AndExpr::AndExpr(left, right) => format!(
                "({} & {})",
                render_and(left, scoped_name, literal),
                render_shift(right, scoped_name, literal)
            ),
        }
    }

    fn render_shift<FScoped, FLit>(
        expr: &hir::ShiftExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&hir::ScopedName) -> String,
        FLit: Fn(&hir::Literal) -> String,
    {
        match expr {
            hir::ShiftExpr::AddExpr(value) => render_add(value, scoped_name, literal),
            hir::ShiftExpr::LeftShiftExpr(left, right) => format!(
                "({} << {})",
                render_shift(left, scoped_name, literal),
                render_add(right, scoped_name, literal)
            ),
            hir::ShiftExpr::RightShiftExpr(left, right) => format!(
                "({} >> {})",
                render_shift(left, scoped_name, literal),
                render_add(right, scoped_name, literal)
            ),
        }
    }

    fn render_add<FScoped, FLit>(
        expr: &hir::AddExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&hir::ScopedName) -> String,
        FLit: Fn(&hir::Literal) -> String,
    {
        match expr {
            hir::AddExpr::MultExpr(value) => render_mult(value, scoped_name, literal),
            hir::AddExpr::AddExpr(left, right) => format!(
                "({} + {})",
                render_add(left, scoped_name, literal),
                render_mult(right, scoped_name, literal)
            ),
            hir::AddExpr::SubExpr(left, right) => format!(
                "({} - {})",
                render_add(left, scoped_name, literal),
                render_mult(right, scoped_name, literal)
            ),
        }
    }

    fn render_mult<FScoped, FLit>(
        expr: &hir::MultExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&hir::ScopedName) -> String,
        FLit: Fn(&hir::Literal) -> String,
    {
        match expr {
            hir::MultExpr::UnaryExpr(value) => render_unary(value, scoped_name, literal),
            hir::MultExpr::MultExpr(left, right) => format!(
                "({} * {})",
                render_mult(left, scoped_name, literal),
                render_unary(right, scoped_name, literal)
            ),
            hir::MultExpr::DivExpr(left, right) => format!(
                "({} / {})",
                render_mult(left, scoped_name, literal),
                render_unary(right, scoped_name, literal)
            ),
            hir::MultExpr::ModExpr(left, right) => format!(
                "({} % {})",
                render_mult(left, scoped_name, literal),
                render_unary(right, scoped_name, literal)
            ),
        }
    }

    fn render_unary<FScoped, FLit>(
        expr: &hir::UnaryExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&hir::ScopedName) -> String,
        FLit: Fn(&hir::Literal) -> String,
    {
        match expr {
            hir::UnaryExpr::PrimaryExpr(value) => render_primary(value, scoped_name, literal),
            hir::UnaryExpr::UnaryExpr(op, value) => {
                let op = match op {
                    hir::UnaryOperator::Add => "+",
                    hir::UnaryOperator::Sub => "-",
                    hir::UnaryOperator::Not => "~",
                };
                format!("({}{})", op, render_primary(value, scoped_name, literal))
            }
        }
    }

    fn render_primary<FScoped, FLit>(
        expr: &hir::PrimaryExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&hir::ScopedName) -> String,
        FLit: Fn(&hir::Literal) -> String,
    {
        match expr {
            hir::PrimaryExpr::ScopedName(value) => scoped_name(value),
            hir::PrimaryExpr::Literal(value) => literal(value),
            hir::PrimaryExpr::ConstExpr(value) => render_const_expr(value, scoped_name, literal),
        }
    }

    render_or(&expr.0, scoped_name, literal)
}
