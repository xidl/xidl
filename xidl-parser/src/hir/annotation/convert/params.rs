//! Raw annotation-text parsing: the source-level parameter text of a
//! raw annotation becomes typed, unescaped parameter entries.

use super::super::{
    BinaryOperator, ConstExpr, IntegerLiteral, IntegerSign, Literal, UnaryOperator,
};

pub(super) fn parse_raw_annotation_params(raw: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            buf.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote.is_some() {
            escaped = true;
            buf.push(ch);
            continue;
        }
        match ch {
            '\'' | '"' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                buf.push(ch);
            }
            ',' if quote.is_none() => {
                let item = buf.trim();
                if !item.is_empty() {
                    parts.push(item.to_string());
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }

    let item = buf.trim();
    if !item.is_empty() {
        parts.push(item.to_string());
    }

    parts
        .into_iter()
        .map(|part| {
            if let Some((key, value)) = part.split_once('=') {
                let value = trim_annotation_quotes(value.trim())
                    .unwrap_or_else(|| value.trim().to_string());
                (key.trim().to_string(), unescape_param_value(&value))
            } else {
                let value =
                    trim_annotation_quotes(part.trim()).unwrap_or_else(|| part.trim().to_string());
                ("value".to_string(), unescape_param_value(&value))
            }
        })
        .collect()
}

fn unescape_param_value(value: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        out.push(ch);
    }
    out
}

pub(super) fn trim_annotation_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() < 2 {
        return None;
    }
    let first = value.chars().next().unwrap();
    let last = value.chars().last().unwrap();
    if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}

pub(crate) fn render_annotation_const_expr(expr: &ConstExpr) -> String {
    match expr {
        ConstExpr::ScopedName(value) => {
            let prefix = if value.is_root { "::" } else { "" };
            format!("{prefix}{}", value.name.join("::"))
        }
        ConstExpr::Literal(value) => render_annotation_literal(value),
        ConstExpr::UnaryExpr(op, value) => {
            let op = match op {
                UnaryOperator::Add => "+",
                UnaryOperator::Sub => "-",
                UnaryOperator::Not => "~",
            };
            format!("({op}{})", render_annotation_const_expr(value))
        }
        ConstExpr::BinaryExpr(op, left, right) => {
            let op = match op {
                BinaryOperator::Or => "|",
                BinaryOperator::Xor => "^",
                BinaryOperator::And => "&",
                BinaryOperator::LeftShift => "<<",
                BinaryOperator::RightShift => ">>",
                BinaryOperator::Add => "+",
                BinaryOperator::Sub => "-",
                BinaryOperator::Mult => "*",
                BinaryOperator::Div => "/",
                BinaryOperator::Mod => "%",
            };
            format!(
                "({} {op} {})",
                render_annotation_const_expr(left),
                render_annotation_const_expr(right)
            )
        }
    }
}

pub(super) fn render_annotation_literal(value: &Literal) -> String {
    match value {
        Literal::IntegerLiteral(IntegerLiteral(value)) => value.clone(),
        Literal::FloatingPtLiteral(value) => {
            let sign = value.sign.as_ref().map(IntegerSign::as_str).unwrap_or("");
            format!("{}{}.{}", sign, value.integer.0, value.fraction.0)
        }
        Literal::CharLiteral(value)
        | Literal::WideCharacterLiteral(value)
        | Literal::StringLiteral(value)
        | Literal::WideStringLiteral(value) => value.clone(),
        Literal::BooleanLiteral(value) => value.to_string(),
    }
}
