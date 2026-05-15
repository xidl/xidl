use super::{Annotation, BinaryOperator, ConstExpr, IntegerLiteral, IntegerSign, Literal, UnaryOperator};

#[cfg(test)]
mod tests;

pub(super) fn from_builtin_annotation(
    value: crate::typed_ast::BuiltinAnnotation,
) -> Option<Annotation> {
    use crate::typed_ast::BuiltinAnnotation as Builtin;

    Some(match value {
        Builtin::Id { value } => Annotation::Id {
            value: integer_literal(value),
        },
        Builtin::AutoId { value } => Annotation::AutoId {
            value: value.map(|value| value.as_str().to_string()),
        },
        Builtin::Optional { value } => Annotation::Optional {
            value: value.map(positive_int_const),
        },
        Builtin::Position { value } => Annotation::Position {
            value: positive_int_const(value),
        },
        Builtin::Value { value } => Annotation::Value {
            value: const_expr(value),
        },
        Builtin::Extensibility { kind } => Annotation::Extensibility {
            kind: kind.as_str().to_string(),
        },
        Builtin::Final => Annotation::Final,
        Builtin::Appendable => Annotation::Appendable,
        Builtin::Mutable => Annotation::Mutable,
        Builtin::Key { value } => Annotation::Key {
            value: value.map(positive_int_const),
        },
        Builtin::MustUnderstand { value } => Annotation::MustUnderstand {
            value: value.map(positive_int_const),
        },
        Builtin::DefaultLiteral => Annotation::DefaultLiteral,
        Builtin::Default { value } => Annotation::Default {
            value: const_expr(value),
        },
        Builtin::Range { min, max } => Annotation::Range {
            min: positive_int_const(min),
            max: positive_int_const(max),
        },
        Builtin::Min { value } => Annotation::Min {
            value: positive_int_const(value),
        },
        Builtin::Max { value } => Annotation::Max {
            value: positive_int_const(value),
        },
        Builtin::Unit { value } => Annotation::Unit {
            value: const_expr(value),
        },
        Builtin::BitBound { value } => Annotation::BitBound {
            value: positive_int_const(value),
        },
        Builtin::External { value } => Annotation::External {
            value: value.map(positive_int_const),
        },
        Builtin::Nested { value } => Annotation::Nested {
            value: value.map(positive_int_const),
        },
        Builtin::Verbatim {
            language,
            placement,
            text,
        } => Annotation::Verbatim {
            language: language.map(|value| value.as_str().to_string()),
            placement: placement.map(|value| value.as_str().to_string()),
            text: const_expr(text),
        },
        Builtin::Service { platform } => Annotation::Service {
            platform: platform.map(|value| value.as_str().to_string()),
        },
        Builtin::Oneway { value } => Annotation::Oneway {
            value: value.map(const_expr),
        },
        Builtin::Ami { value } => Annotation::Ami {
            value: value.map(const_expr),
        },
        Builtin::HashId { value } => Annotation::HashId {
            value: value.map(const_expr),
        },
        Builtin::DefaultNested { value } => Annotation::DefaultNested {
            value: value.map(const_expr),
        },
        Builtin::IgnoreLiteralNames { value } => Annotation::IgnoreLiteralNames {
            value: value.map(const_expr),
        },
        Builtin::TryConstruct { value } => Annotation::TryConstruct {
            value: value.map(|value| value.as_str().to_string()),
        },
        Builtin::NonSerialized { value } => Annotation::NonSerialized {
            value: value.map(boolean_literal),
        },
        Builtin::DataRepresentation { kinds } => Annotation::DataRepresentation {
            kinds: kinds
                .into_iter()
                .map(|value| value.as_str().to_string())
                .collect(),
        },
        Builtin::Topic { name, platform } => Annotation::Topic {
            name: name.map(const_expr),
            platform: platform.map(|value| value.as_str().to_string()),
        },
        Builtin::Choice => Annotation::Choice,
        Builtin::Empty => Annotation::Empty,
        Builtin::DdsService => Annotation::DdsService,
        Builtin::DdsRequestTopic { name } => Annotation::DdsRequestTopic {
            name: const_expr(name),
        },
        Builtin::DdsReplyTopic { name } => Annotation::DdsReplyTopic {
            name: const_expr(name),
        },
        Builtin::Rename { .. }
        | Builtin::SerializeName { .. }
        | Builtin::DeserializeName { .. }
        | Builtin::RenameAll { .. } => return None,
    })
}

fn const_expr(value: crate::typed_ast::ConstExpr) -> String {
    render_hir_const_expr(&value.into())
}

fn positive_int_const(value: crate::typed_ast::PositiveIntConst) -> String {
    const_expr(value.0)
}

fn integer_literal(value: crate::typed_ast::IntegerLiteral) -> String {
    match value {
        crate::typed_ast::IntegerLiteral::BinNumber(value)
        | crate::typed_ast::IntegerLiteral::OctNumber(value)
        | crate::typed_ast::IntegerLiteral::DecNumber(value)
        | crate::typed_ast::IntegerLiteral::HexNumber(value) => value,
    }
}

fn boolean_literal(value: crate::typed_ast::BooleanLiteral) -> String {
    value.as_bool().to_string()
}

fn render_hir_const_expr(expr: &ConstExpr) -> String {
    match expr {
        ConstExpr::ScopedName(value) => {
            let prefix = if value.is_root { "::" } else { "" };
            format!("{prefix}{}", value.name.join("::"))
        }
        ConstExpr::Literal(value) => render_literal(value),
        ConstExpr::UnaryExpr(op, value) => {
            let op = match op {
                UnaryOperator::Add => "+",
                UnaryOperator::Sub => "-",
                UnaryOperator::Not => "~",
            };
            format!("({op}{})", render_hir_const_expr(value))
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
                render_hir_const_expr(left),
                render_hir_const_expr(right)
            )
        }
    }
}

fn render_literal(value: &Literal) -> String {
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
