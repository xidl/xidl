use super::*;

fn int_expr(literal: crate::typed_ast::IntegerLiteral) -> crate::typed_ast::ConstExpr {
    use crate::typed_ast::{
        AddExpr, AndExpr, ConstExpr, Literal, MultExpr, OrExpr, PrimaryExpr, ShiftExpr, UnaryExpr,
        XorExpr,
    };

    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(PrimaryExpr::Literal(Literal::IntegerLiteral(literal))),
        ))),
    ))))
}

fn binary_expr() -> crate::typed_ast::ConstExpr {
    crate::typed_ast::ConstExpr(crate::typed_ast::OrExpr::OrExpr(
        Box::new(crate::typed_ast::OrExpr::XorExpr(
            crate::typed_ast::XorExpr::AndExpr(crate::typed_ast::AndExpr::ShiftExpr(
                crate::typed_ast::ShiftExpr::AddExpr(crate::typed_ast::AddExpr::MultExpr(
                    crate::typed_ast::MultExpr::UnaryExpr(
                        crate::typed_ast::UnaryExpr::PrimaryExpr(
                            crate::typed_ast::PrimaryExpr::Literal(
                                crate::typed_ast::Literal::IntegerLiteral(
                                    crate::typed_ast::IntegerLiteral::DecNumber("1".to_string()),
                                ),
                            ),
                        ),
                    ),
                )),
            )),
        )),
        crate::typed_ast::XorExpr::AndExpr(crate::typed_ast::AndExpr::ShiftExpr(
            crate::typed_ast::ShiftExpr::AddExpr(crate::typed_ast::AddExpr::MultExpr(
                crate::typed_ast::MultExpr::UnaryExpr(crate::typed_ast::UnaryExpr::PrimaryExpr(
                    crate::typed_ast::PrimaryExpr::Literal(
                        crate::typed_ast::Literal::IntegerLiteral(
                            crate::typed_ast::IntegerLiteral::DecNumber("2".to_string()),
                        ),
                    ),
                )),
            )),
        )),
    ))
}

#[test]
fn const_expr_to_i64_handles_literals_and_rejects_composed_forms() {
    assert_eq!(
        const_expr_to_i64(
            &int_expr(crate::typed_ast::IntegerLiteral::BinNumber(
                "0B1010".to_string(),
            ))
            .into()
        ),
        Some(10)
    );
    assert_eq!(
        const_expr_to_i64(
            &int_expr(crate::typed_ast::IntegerLiteral::OctNumber(
                "0o17".to_string(),
            ))
            .into()
        ),
        Some(15)
    );
    assert_eq!(
        const_expr_to_i64(
            &int_expr(crate::typed_ast::IntegerLiteral::DecNumber(
                "42".to_string(),
            ))
            .into()
        ),
        Some(42)
    );
    assert_eq!(
        const_expr_to_i64(
            &int_expr(crate::typed_ast::IntegerLiteral::HexNumber(
                "0x1f".to_string(),
            ))
            .into()
        ),
        Some(31)
    );

    assert_eq!(const_expr_to_i64(&binary_expr().into()), None);
    let negative = crate::typed_ast::ConstExpr(crate::typed_ast::OrExpr::XorExpr(
        crate::typed_ast::XorExpr::AndExpr(crate::typed_ast::AndExpr::ShiftExpr(
            crate::typed_ast::ShiftExpr::AddExpr(crate::typed_ast::AddExpr::MultExpr(
                crate::typed_ast::MultExpr::UnaryExpr(crate::typed_ast::UnaryExpr::UnaryExpr(
                    crate::typed_ast::UnaryOperator::Sub,
                    crate::typed_ast::PrimaryExpr::Literal(
                        crate::typed_ast::Literal::IntegerLiteral(
                            crate::typed_ast::IntegerLiteral::DecNumber("7".to_string()),
                        ),
                    ),
                )),
            )),
        )),
    ));
    assert_eq!(const_expr_to_i64(&negative.into()), Some(-7));
}

#[test]
fn conversion_covers_recursive_expr_and_literal_variants() {
    let scoped = crate::typed_ast::ScopedName {
        scoped_name: Some(Box::new(crate::typed_ast::ScopedName {
            scoped_name: None,
            identifier: crate::typed_ast::Identifier("demo".to_string()),
            node_text: "demo".to_string(),
        })),
        identifier: crate::typed_ast::Identifier("VALUE".to_string()),
        node_text: "::demo::VALUE".to_string(),
    };
    let nested = crate::typed_ast::ConstExpr(crate::typed_ast::OrExpr::XorExpr(
        crate::typed_ast::XorExpr::XorExpr(
            Box::new(crate::typed_ast::XorExpr::AndExpr(crate::typed_ast::AndExpr::AndExpr(
                Box::new(crate::typed_ast::AndExpr::ShiftExpr(
                    crate::typed_ast::ShiftExpr::LeftShiftExpr(
                        Box::new(crate::typed_ast::ShiftExpr::RightShiftExpr(
                            Box::new(crate::typed_ast::ShiftExpr::AddExpr(
                                crate::typed_ast::AddExpr::AddExpr(
                                    Box::new(crate::typed_ast::AddExpr::SubExpr(
                                        Box::new(crate::typed_ast::AddExpr::MultExpr(
                                            crate::typed_ast::MultExpr::MultExpr(
                                                Box::new(crate::typed_ast::MultExpr::DivExpr(
                                                    Box::new(crate::typed_ast::MultExpr::ModExpr(
                                                        Box::new(crate::typed_ast::MultExpr::UnaryExpr(
                                                            crate::typed_ast::UnaryExpr::UnaryExpr(
                                                                crate::typed_ast::UnaryOperator::Add,
                                                                crate::typed_ast::PrimaryExpr::ConstExpr(
                                                                    Box::new(int_expr(
                                                                        crate::typed_ast::IntegerLiteral::BinNumber(
                                                                            "0b1".to_string(),
                                                                        ),
                                                                    )),
                                                                ),
                                                            ),
                                                        )),
                                                        crate::typed_ast::UnaryExpr::PrimaryExpr(
                                                            crate::typed_ast::PrimaryExpr::Literal(
                                                                crate::typed_ast::Literal::FloatingPtLiteral(
                                                                    crate::typed_ast::FloatingPtLiteral {
                                                                        sign: Some(crate::typed_ast::IntegerSign::Minus),
                                                                        integer: crate::typed_ast::DecNumber(
                                                                            "1".to_string(),
                                                                        ),
                                                                        fraction: crate::typed_ast::DecNumber(
                                                                            "5".to_string(),
                                                                        ),
                                                                    },
                                                                ),
                                                            ),
                                                        ),
                                                    )),
                                                    crate::typed_ast::UnaryExpr::PrimaryExpr(
                                                        crate::typed_ast::PrimaryExpr::Literal(
                                                            crate::typed_ast::Literal::CharLiteral(
                                                                "'a'".to_string(),
                                                            ),
                                                        ),
                                                    ),
                                                )),
                                                crate::typed_ast::UnaryExpr::PrimaryExpr(
                                                    crate::typed_ast::PrimaryExpr::Literal(
                                                        crate::typed_ast::Literal::WideCharacterLiteral(
                                                            "L'a'".to_string(),
                                                        ),
                                                    ),
                                                ),
                                            ),
                                        )),
                                        crate::typed_ast::MultExpr::UnaryExpr(
                                            crate::typed_ast::UnaryExpr::PrimaryExpr(
                                                crate::typed_ast::PrimaryExpr::Literal(
                                                    crate::typed_ast::Literal::StringLiteral(
                                                        "\"hi\"".to_string(),
                                                    ),
                                                ),
                                            ),
                                        ),
                                    )),
                                    crate::typed_ast::MultExpr::UnaryExpr(
                                        crate::typed_ast::UnaryExpr::PrimaryExpr(
                                            crate::typed_ast::PrimaryExpr::Literal(
                                                crate::typed_ast::Literal::WideStringLiteral(
                                                    "L\"hi\"".to_string(),
                                                ),
                                            ),
                                        ),
                                    ),
                                ),
                            )),
                            crate::typed_ast::AddExpr::MultExpr(crate::typed_ast::MultExpr::UnaryExpr(
                                crate::typed_ast::UnaryExpr::PrimaryExpr(
                                    crate::typed_ast::PrimaryExpr::Literal(
                                        crate::typed_ast::Literal::BooleanLiteral(
                                            crate::typed_ast::BooleanLiteral::True,
                                        ),
                                    ),
                                ),
                            )),
                        )),
                        crate::typed_ast::AddExpr::MultExpr(crate::typed_ast::MultExpr::UnaryExpr(
                            crate::typed_ast::UnaryExpr::PrimaryExpr(
                                crate::typed_ast::PrimaryExpr::ScopedName(scoped.clone()),
                            ),
                        )),
                    ),
                )),
                crate::typed_ast::ShiftExpr::AddExpr(crate::typed_ast::AddExpr::MultExpr(
                    crate::typed_ast::MultExpr::UnaryExpr(crate::typed_ast::UnaryExpr::UnaryExpr(
                        crate::typed_ast::UnaryOperator::Not,
                        crate::typed_ast::PrimaryExpr::Literal(crate::typed_ast::Literal::IntegerLiteral(
                            crate::typed_ast::IntegerLiteral::HexNumber("0x1".to_string()),
                        )),
                    )),
                )),
            ))),
            crate::typed_ast::AndExpr::ShiftExpr(crate::typed_ast::ShiftExpr::AddExpr(
                crate::typed_ast::AddExpr::MultExpr(crate::typed_ast::MultExpr::UnaryExpr(
                    crate::typed_ast::UnaryExpr::PrimaryExpr(crate::typed_ast::PrimaryExpr::Literal(
                        crate::typed_ast::Literal::IntegerLiteral(
                            crate::typed_ast::IntegerLiteral::OctNumber("0o7".to_string()),
                        ),
                    )),
                )),
            )),
        ),
    ));
    let converted: ConstExpr = nested.into();
    assert_eq!(const_expr_to_i64(&converted), None);
}
