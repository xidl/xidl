use idl_rs::typed_ast;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub filename: String,
    pub filecontent: String,
}

pub mod c;

pub fn to_snake_case(input: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in input.chars() {
        if ch.is_ascii_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_lower = false;
        } else if ch == '-' || ch == ' ' {
            out.push('_');
            prev_lower = false;
        } else {
            out.push(ch.to_ascii_lowercase());
            prev_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }
    out
}

pub fn to_pascal_case(input: &str) -> String {
    if !input.contains('_') && !input.contains('-') {
        return input.to_string();
    }
    let mut out = String::new();
    for part in input.split(|ch| ch == '_' || ch == '-') {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            for ch in chars {
                out.push(ch.to_ascii_lowercase());
            }
        }
    }
    out
}

pub fn to_upper_snake_case(input: &str) -> String {
    let snake = to_snake_case(input);
    snake.to_ascii_uppercase()
}

pub fn typed_scoped_name_parts(scoped: &typed_ast::ScopedName) -> Vec<String> {
    fn collect(scoped: &typed_ast::ScopedName, out: &mut Vec<String>) {
        if let Some(next) = &scoped.scoped_name {
            collect(next, out);
        }
        out.push(scoped.identifier.0.clone());
    }

    let mut parts = Vec::new();
    collect(scoped, &mut parts);
    parts
}

pub fn render_const_expr<FScoped, FLit>(
    expr: &typed_ast::ConstExpr,
    scoped_name: &FScoped,
    literal: &FLit,
) -> String
where
    FScoped: Fn(&typed_ast::ScopedName) -> String,
    FLit: Fn(&typed_ast::Literal) -> String,
{
    fn render_or<FScoped, FLit>(
        expr: &typed_ast::OrExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&typed_ast::ScopedName) -> String,
        FLit: Fn(&typed_ast::Literal) -> String,
    {
        match expr {
            typed_ast::OrExpr::XorExpr(value) => render_xor(value, scoped_name, literal),
            typed_ast::OrExpr::OrExpr(left, right) => format!(
                "({} | {})",
                render_or(left, scoped_name, literal),
                render_xor(right, scoped_name, literal)
            ),
        }
    }

    fn render_xor<FScoped, FLit>(
        expr: &typed_ast::XorExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&typed_ast::ScopedName) -> String,
        FLit: Fn(&typed_ast::Literal) -> String,
    {
        match expr {
            typed_ast::XorExpr::AndExpr(value) => render_and(value, scoped_name, literal),
            typed_ast::XorExpr::XorExpr(left, right) => format!(
                "({} ^ {})",
                render_xor(left, scoped_name, literal),
                render_and(right, scoped_name, literal)
            ),
        }
    }

    fn render_and<FScoped, FLit>(
        expr: &typed_ast::AndExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&typed_ast::ScopedName) -> String,
        FLit: Fn(&typed_ast::Literal) -> String,
    {
        match expr {
            typed_ast::AndExpr::ShiftExpr(value) => render_shift(value, scoped_name, literal),
            typed_ast::AndExpr::AndExpr(left, right) => format!(
                "({} & {})",
                render_and(left, scoped_name, literal),
                render_shift(right, scoped_name, literal)
            ),
        }
    }

    fn render_shift<FScoped, FLit>(
        expr: &typed_ast::ShiftExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&typed_ast::ScopedName) -> String,
        FLit: Fn(&typed_ast::Literal) -> String,
    {
        match expr {
            typed_ast::ShiftExpr::AddExpr(value) => render_add(value, scoped_name, literal),
            typed_ast::ShiftExpr::LeftShiftExpr(left, right) => format!(
                "({} << {})",
                render_shift(left, scoped_name, literal),
                render_add(right, scoped_name, literal)
            ),
            typed_ast::ShiftExpr::RightShiftExpr(left, right) => format!(
                "({} >> {})",
                render_shift(left, scoped_name, literal),
                render_add(right, scoped_name, literal)
            ),
        }
    }

    fn render_add<FScoped, FLit>(
        expr: &typed_ast::AddExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&typed_ast::ScopedName) -> String,
        FLit: Fn(&typed_ast::Literal) -> String,
    {
        match expr {
            typed_ast::AddExpr::MultExpr(value) => render_mult(value, scoped_name, literal),
            typed_ast::AddExpr::AddExpr(left, right) => format!(
                "({} + {})",
                render_add(left, scoped_name, literal),
                render_mult(right, scoped_name, literal)
            ),
            typed_ast::AddExpr::SubExpr(left, right) => format!(
                "({} - {})",
                render_add(left, scoped_name, literal),
                render_mult(right, scoped_name, literal)
            ),
        }
    }

    fn render_mult<FScoped, FLit>(
        expr: &typed_ast::MultExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&typed_ast::ScopedName) -> String,
        FLit: Fn(&typed_ast::Literal) -> String,
    {
        match expr {
            typed_ast::MultExpr::UnaryExpr(value) => render_unary(value, scoped_name, literal),
            typed_ast::MultExpr::MultExpr(left, right) => format!(
                "({} * {})",
                render_mult(left, scoped_name, literal),
                render_unary(right, scoped_name, literal)
            ),
            typed_ast::MultExpr::DivExpr(left, right) => format!(
                "({} / {})",
                render_mult(left, scoped_name, literal),
                render_unary(right, scoped_name, literal)
            ),
            typed_ast::MultExpr::ModExpr(left, right) => format!(
                "({} % {})",
                render_mult(left, scoped_name, literal),
                render_unary(right, scoped_name, literal)
            ),
        }
    }

    fn render_unary<FScoped, FLit>(
        expr: &typed_ast::UnaryExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&typed_ast::ScopedName) -> String,
        FLit: Fn(&typed_ast::Literal) -> String,
    {
        match expr {
            typed_ast::UnaryExpr::PrimaryExpr(value) => render_primary(value, scoped_name, literal),
            typed_ast::UnaryExpr::UnaryExpr(op, value) => {
                let op = match op {
                    typed_ast::UnaryOperator::Add => "+",
                    typed_ast::UnaryOperator::Sub => "-",
                    typed_ast::UnaryOperator::Not => "~",
                };
                format!("({}{})", op, render_primary(value, scoped_name, literal))
            }
        }
    }

    fn render_primary<FScoped, FLit>(
        expr: &typed_ast::PrimaryExpr,
        scoped_name: &FScoped,
        literal: &FLit,
    ) -> String
    where
        FScoped: Fn(&typed_ast::ScopedName) -> String,
        FLit: Fn(&typed_ast::Literal) -> String,
    {
        match expr {
            typed_ast::PrimaryExpr::ScopedName(value) => scoped_name(value),
            typed_ast::PrimaryExpr::Literal(value) => literal(value),
            typed_ast::PrimaryExpr::ConstExpr(value) => render_const_expr(value, scoped_name, literal),
        }
    }

    render_or(&expr.0, scoped_name, literal)
}
