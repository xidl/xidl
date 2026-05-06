use super::*;
use crate::hir::{
    BinaryOperator, ConstExpr, DecNumber, FloatingPtLiteral, IntegerLiteral, IntegerSign, Literal,
    ScopedName, UnaryOperator,
};

fn int(value: &str) -> ConstExpr {
    ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(value.to_string())))
}

#[test]
fn parse_string_array_and_raw_params_handle_nested_values() {
    assert_eq!(
        parse_string_array(r#""a,b", call(x,y), [left,right], plain, 'x,y'"#),
        vec![
            "\"a,b\"".to_string(),
            "call(x,y)".to_string(),
            "[left,right]".to_string(),
            "plain".to_string(),
            "'x,y'".to_string(),
        ]
    );

    assert_eq!(parse_raw_params(""), Vec::<(String, String)>::new());
    assert_eq!(
        parse_raw_params(r#""city-id""#),
        vec![("value".to_string(), "city-id".to_string())]
    );
    assert_eq!(
        parse_raw_params(r#"name="X-Trace", scopes=["read","write"], value='literal'"#),
        vec![
            ("name".to_string(), "X-Trace".to_string()),
            ("scopes".to_string(), "[\"read\",\"write\"]".to_string()),
            ("value".to_string(), "literal".to_string()),
        ]
    );
}

#[test]
fn trim_quotes_only_accepts_balanced_single_or_double_quotes() {
    assert_eq!(trim_quotes(r#""hello""#).as_deref(), Some("hello"));
    assert_eq!(trim_quotes("'world'").as_deref(), Some("world"));
    assert_eq!(trim_quotes("plain"), None);
    assert_eq!(trim_quotes("'unterminated"), None);
}

#[test]
fn render_const_expr_covers_scoped_names_literals_unary_and_binary_ops() {
    let scoped = ConstExpr::ScopedName(ScopedName {
        name: vec!["MyModule".to_string(), "MyValue".to_string()],
        is_root: true,
    });
    assert_eq!(render_const_expr(&scoped), "::my_module::my_value");

    let float = ConstExpr::Literal(Literal::FloatingPtLiteral(FloatingPtLiteral {
        sign: Some(IntegerSign::Minus),
        integer: DecNumber("12".to_string()),
        fraction: DecNumber("5".to_string()),
    }));
    assert_eq!(render_const_expr(&float), "-12.5");
    assert_eq!(
        render_const_expr(&ConstExpr::Literal(Literal::CharLiteral("'x'".to_string()))),
        "'x'"
    );
    assert_eq!(
        render_const_expr(&ConstExpr::Literal(Literal::WideCharacterLiteral(
            "L'x'".to_string()
        ))),
        "L'x'"
    );
    assert_eq!(
        render_const_expr(&ConstExpr::Literal(Literal::StringLiteral(
            "\"hi\"".to_string()
        ))),
        "\"hi\""
    );
    assert_eq!(
        render_const_expr(&ConstExpr::Literal(Literal::WideStringLiteral(
            "L\"hi\"".to_string()
        ))),
        "L\"hi\""
    );
    assert_eq!(
        render_const_expr(&ConstExpr::Literal(Literal::BooleanLiteral(true))),
        "true"
    );

    let unary_cases = [
        (UnaryOperator::Add, "(+1)"),
        (UnaryOperator::Sub, "(-1)"),
        (UnaryOperator::Not, "(~1)"),
    ];
    for (op, expected) in unary_cases {
        let expr = ConstExpr::UnaryExpr(op, Box::new(int("1")));
        assert_eq!(render_const_expr(&expr), expected);
    }

    let binary_cases = [
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
    ];
    for (op, expected) in binary_cases {
        let expr = ConstExpr::BinaryExpr(op, Box::new(int("1")), Box::new(int("2")));
        assert_eq!(render_const_expr(&expr), expected);
    }
}
