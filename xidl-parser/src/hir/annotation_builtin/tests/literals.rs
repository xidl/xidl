use super::*;
use crate::hir::{
    BinaryOperator, ConstExpr, DecNumber, FloatingPtLiteral, IntegerLiteral, IntegerSign, Literal,
    ScopedName, UnaryOperator,
};

#[test]
fn renders_all_literal_forms_when_converting_builtin_values() {
    assert_eq!(
        from_builtin_annotation(BuiltinAnnotation::Id {
            value: TypedIntegerLiteral::OctNumber("0o77".to_string()),
        })
        .and_then(|value| match value {
            Annotation::Id { value } => Some(value),
            _ => None,
        })
        .as_deref(),
        Some("0o77")
    );
    let float = render_hir_const_expr(&ConstExpr::Literal(Literal::FloatingPtLiteral(
        FloatingPtLiteral {
            sign: Some(IntegerSign::Minus),
            integer: DecNumber("12".to_string()),
            fraction: DecNumber("5".to_string()),
        },
    )));
    assert_eq!(float, "-12.5");
    assert_eq!(
        render_hir_const_expr(&ConstExpr::Literal(Literal::CharLiteral("'x'".to_string()))),
        "'x'"
    );
    assert_eq!(
        render_hir_const_expr(&ConstExpr::Literal(Literal::WideCharacterLiteral(
            "L'x'".to_string()
        ))),
        "L'x'"
    );
    assert_eq!(
        render_hir_const_expr(&ConstExpr::Literal(Literal::WideStringLiteral(
            "L\"wide\"".to_string()
        ))),
        "L\"wide\""
    );
    assert_eq!(
        render_hir_const_expr(&ConstExpr::ScopedName(ScopedName {
            name: vec!["pkg".to_string(), "Value".to_string()],
            is_root: true,
        })),
        "::pkg::Value"
    );
    assert_eq!(
        render_hir_const_expr(&ConstExpr::UnaryExpr(
            UnaryOperator::Add,
            Box::new(ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(
                "1".to_string(),
            )))),
        )),
        "(+1)"
    );
    assert_eq!(
        render_hir_const_expr(&ConstExpr::UnaryExpr(
            UnaryOperator::Not,
            Box::new(ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(
                "1".to_string(),
            )))),
        )),
        "(~1)"
    );
    for (op, expected) in [
        (BinaryOperator::Or, "(1 | 2)"),
        (BinaryOperator::Xor, "(1 ^ 2)"),
        (BinaryOperator::And, "(1 & 2)"),
        (BinaryOperator::LeftShift, "(1 << 2)"),
        (BinaryOperator::RightShift, "(1 >> 2)"),
        (BinaryOperator::Add, "(1 + 2)"),
        (BinaryOperator::Sub, "(1 - 2)"),
        (BinaryOperator::Mult, "(1 * 2)"),
        (BinaryOperator::Div, "(1 / 2)"),
        (BinaryOperator::Mod, "(1 % 2)"),
    ] {
        assert_eq!(
            render_hir_const_expr(&ConstExpr::BinaryExpr(
                op,
                Box::new(ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(
                    "1".to_string(),
                )))),
                Box::new(ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(
                    "2".to_string(),
                )))),
            )),
            expected
        );
    }
}
