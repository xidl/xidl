//! Deterministic rendering of HIR type and HTTP enum values.

use crate::hir::{self, TypeSpec};
use crate::rest_hir::{HttpMethod, HttpParamKind};
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

pub(super) fn render_switch_type(value: &hir::SwitchTypeSpec) -> String {
    match value {
        hir::SwitchTypeSpec::IntegerType(value) => render_integer_type(value),
        hir::SwitchTypeSpec::CharType => "char".to_string(),
        hir::SwitchTypeSpec::WideCharType => "wchar".to_string(),
        hir::SwitchTypeSpec::BooleanType => "boolean".to_string(),
        hir::SwitchTypeSpec::ScopedName(value) => render_scoped_name(value),
        hir::SwitchTypeSpec::OctetType => "octet".to_string(),
    }
}

fn render_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "char",
        hir::IntegerType::UChar => "unsigned char",
        hir::IntegerType::Octet => "octet",
        hir::IntegerType::U8 => "uint8",
        hir::IntegerType::U16 => "unsigned short",
        hir::IntegerType::U32 => "unsigned long",
        hir::IntegerType::U64 => "unsigned long long",
        hir::IntegerType::I8 => "int8",
        hir::IntegerType::I16 => "short",
        hir::IntegerType::I32 => "long",
        hir::IntegerType::I64 => "long long",
    }
    .to_string()
}

pub(super) fn render_bitfield_type(value: &hir::BitFieldType) -> String {
    match value {
        hir::BitFieldType::Bool => "boolean",
        hir::BitFieldType::Octec => "octet",
        hir::BitFieldType::SignedInt => "signed int",
        hir::BitFieldType::UnsignedInt => "unsigned int",
    }
    .to_string()
}

pub(super) fn render_scoped_name(value: &hir::ScopedName) -> String {
    let prefix = if value.is_root { "::" } else { "" };
    format!("{prefix}{}", value.name.join("::"))
}
pub(super) fn declarator_ident(value: &hir::Declarator) -> &str {
    match value {
        hir::Declarator::SimpleDeclarator(value) => &value.0,
        hir::Declarator::ArrayDeclarator(value) => &value.ident,
    }
}

pub(super) fn declarator_facts(value: &hir::Declarator) -> BTreeMap<String, Value> {
    match value {
        hir::Declarator::SimpleDeclarator(value) => {
            BTreeMap::from([("name".to_string(), Value::String(value.0.clone()))])
        }
        hir::Declarator::ArrayDeclarator(value) => BTreeMap::from([
            ("name".to_string(), Value::String(value.ident.clone())),
            ("array".to_string(), Value::Bool(true)),
            (
                "lengths".to_string(),
                Value::Array(
                    value
                        .len
                        .iter()
                        .map(render_positive_int)
                        .map(Value::String)
                        .collect(),
                ),
            ),
        ]),
    }
}

pub(super) fn render_element_type(value: &hir::ElementSpecTy) -> String {
    match value {
        hir::ElementSpecTy::TypeSpec(value) => render_type(value),
        hir::ElementSpecTy::ConstrTypeDcl(value) => value.ident().to_string(),
    }
}

pub(super) fn render_type(value: &TypeSpec) -> String {
    match value {
        TypeSpec::IntegerType(value) => match value {
            hir::IntegerType::Char => "char",
            hir::IntegerType::UChar | hir::IntegerType::Octet | hir::IntegerType::U8 => "uint8",
            hir::IntegerType::U16 => "unsigned short",
            hir::IntegerType::U32 => "unsigned long",
            hir::IntegerType::U64 => "unsigned long long",
            hir::IntegerType::I8 => "int8",
            hir::IntegerType::I16 => "short",
            hir::IntegerType::I32 => "long",
            hir::IntegerType::I64 => "long long",
        }
        .to_string(),
        TypeSpec::FloatingPtType => "double".to_string(),
        TypeSpec::CharType => "char".to_string(),
        TypeSpec::WideCharType => "wchar".to_string(),
        TypeSpec::Boolean => "boolean".to_string(),
        TypeSpec::AnyType => "any".to_string(),
        TypeSpec::ObjectType => "Object".to_string(),
        TypeSpec::ValueBaseType => "ValueBase".to_string(),
        TypeSpec::ScopedName(value) => render_scoped_name(value),
        TypeSpec::SequenceType(value) => {
            render_template_type("sequence", &[render_type(&value.ty)], value.len.as_ref())
        }
        TypeSpec::StringType(value) => render_bounded_type("string", value.bound.as_ref()),
        TypeSpec::WideStringType(value) => render_bounded_type("wstring", value.bound.as_ref()),
        TypeSpec::FixedPtType(value) => format!(
            "fixed<{}, {}>",
            render_positive_int(&value.integer),
            render_positive_int(&value.fraction)
        ),
        TypeSpec::MapType(value) => format!(
            "map<{}>",
            [render_type(&value.key), render_type(&value.value)]
                .into_iter()
                .chain(value.len.as_ref().map(render_positive_int))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            value.ident,
            value
                .args
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn render_template_type(
    name: &str,
    args: &[String],
    bound: Option<&hir::PositiveIntConst>,
) -> String {
    let mut args = args.to_vec();
    if let Some(bound) = bound {
        args.push(render_positive_int(bound));
    }
    format!("{name}<{}>", args.join(", "))
}

fn render_bounded_type(name: &str, bound: Option<&hir::PositiveIntConst>) -> String {
    bound
        .map(|bound| format!("{name}<{}>", render_positive_int(bound)))
        .unwrap_or_else(|| name.to_string())
}

fn render_positive_int(value: &hir::PositiveIntConst) -> String {
    render_const_expr(&value.0)
}

fn render_const_expr(value: &hir::ConstExpr) -> String {
    match value {
        hir::ConstExpr::ScopedName(value) => render_scoped_name(value),
        hir::ConstExpr::Literal(value) => render_literal(value),
        hir::ConstExpr::UnaryExpr(operator, value) => {
            format!(
                "{}{}",
                render_unary_operator(operator),
                render_const_expr(value)
            )
        }
        hir::ConstExpr::BinaryExpr(operator, left, right) => format!(
            "({} {} {})",
            render_const_expr(left),
            render_binary_operator(operator),
            render_const_expr(right)
        ),
    }
}

fn render_literal(value: &hir::Literal) -> String {
    match value {
        hir::Literal::IntegerLiteral(value) => value.0.clone(),
        hir::Literal::FloatingPtLiteral(value) => format!(
            "{}{}.{}",
            value.sign.as_ref().map_or("", hir::IntegerSign::as_str),
            value.integer.0,
            value.fraction.0
        ),
        hir::Literal::CharLiteral(value)
        | hir::Literal::WideCharacterLiteral(value)
        | hir::Literal::StringLiteral(value)
        | hir::Literal::WideStringLiteral(value) => value.clone(),
        hir::Literal::BooleanLiteral(value) => value.to_string(),
    }
}

fn render_unary_operator(value: &hir::UnaryOperator) -> &'static str {
    match value {
        hir::UnaryOperator::Add => "+",
        hir::UnaryOperator::Sub => "-",
        hir::UnaryOperator::Not => "!",
    }
}

fn render_binary_operator(value: &hir::BinaryOperator) -> &'static str {
    match value {
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
    }
}

/// Canonical location spelling used by parameter pairing keys.
pub(in crate::versioning::map) fn render_param_kind(kind: HttpParamKind) -> &'static str {
    match kind {
        HttpParamKind::Path => "path",
        HttpParamKind::Query => "query",
        HttpParamKind::Header => "header",
        HttpParamKind::Cookie => "cookie",
        HttpParamKind::Body => "body",
    }
}

pub(super) fn render_method(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Patch => "patch",
        HttpMethod::Delete => "delete",
        HttpMethod::Head => "head",
        HttpMethod::Options => "options",
    }
    .to_string()
}

pub(super) fn join(parent: &str, ident: &str) -> String {
    if parent.is_empty() {
        ident.to_string()
    } else {
        format!("{parent}::{ident}")
    }
}
