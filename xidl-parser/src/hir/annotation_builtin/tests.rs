use super::*;
use crate::typed_ast::{
    AddExpr, AndExpr, AnnotationAppl, AnnotationName, AutoIdKind, BooleanLiteral,
    BuiltinAnnotation, ConstExpr as TypedConstExpr, DataRepresentationKind, ExtensibilityKind,
    Identifier, IntegerLiteral as TypedIntegerLiteral, Literal as TypedLiteral, MultExpr, OrExpr,
    PlacementKind, PositiveIntConst, PrimaryExpr, ScopedName as TypedScopedName, ServicePlatform,
    ShiftExpr, TopicPlatform, TryConstructFailAction, UnaryExpr,
    UnaryOperator as TypedUnaryOperator, VerbatimLanguage, XorExpr,
};

fn int_expr(value: &str) -> TypedConstExpr {
    TypedConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(PrimaryExpr::Literal(TypedLiteral::IntegerLiteral(
                TypedIntegerLiteral::DecNumber(value.to_string()),
            ))),
        ))),
    ))))
}

fn string_expr(value: &str) -> TypedConstExpr {
    TypedConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(PrimaryExpr::Literal(TypedLiteral::StringLiteral(
                value.to_string(),
            ))),
        ))),
    ))))
}

fn bool_expr(value: bool) -> TypedConstExpr {
    TypedConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(PrimaryExpr::Literal(TypedLiteral::BooleanLiteral(
                if value {
                    BooleanLiteral::True
                } else {
                    BooleanLiteral::False
                },
            ))),
        ))),
    ))))
}

fn scoped_expr(is_root: bool, parts: &[&str]) -> TypedConstExpr {
    let mut scoped = None;
    for part in parts {
        let text = if scoped.is_none() && is_root {
            format!("::{part}")
        } else {
            part.to_string()
        };
        scoped = Some(TypedScopedName {
            scoped_name: scoped.map(Box::new),
            identifier: Identifier((*part).to_string()),
            node_text: text,
        });
    }
    TypedConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(PrimaryExpr::ScopedName(scoped.expect("scoped name"))),
        ))),
    ))))
}

fn unary_expr(op: TypedUnaryOperator, value: TypedConstExpr) -> TypedConstExpr {
    let primary = match value.0 {
        OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(ShiftExpr::AddExpr(
            AddExpr::MultExpr(MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(primary))),
        )))) => primary,
        other => PrimaryExpr::ConstExpr(Box::new(TypedConstExpr(other))),
    };
    TypedConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::UnaryExpr(op, primary),
        ))),
    ))))
}

#[test]
fn maps_builtin_annotations_to_hir_variants() {
    let cases = vec![
        (
            BuiltinAnnotation::Id {
                value: TypedIntegerLiteral::HexNumber("0x10".to_string()),
            },
            Annotation::Id {
                value: "0x10".to_string(),
            },
        ),
        (
            BuiltinAnnotation::AutoId {
                value: Some(AutoIdKind::Hash),
            },
            Annotation::AutoId {
                value: Some("HASH".to_string()),
            },
        ),
        (
            BuiltinAnnotation::Optional {
                value: Some(PositiveIntConst(int_expr("2"))),
            },
            Annotation::Optional {
                value: Some("2".to_string()),
            },
        ),
        (
            BuiltinAnnotation::Position {
                value: PositiveIntConst(int_expr("3")),
            },
            Annotation::Position {
                value: "3".to_string(),
            },
        ),
        (
            BuiltinAnnotation::Value {
                value: unary_expr(TypedUnaryOperator::Sub, int_expr("7")),
            },
            Annotation::Value {
                value: "(-7)".to_string(),
            },
        ),
        (
            BuiltinAnnotation::Extensibility {
                kind: ExtensibilityKind::Mutable,
            },
            Annotation::Extensibility {
                kind: "MUTABLE".to_string(),
            },
        ),
        (BuiltinAnnotation::Final, Annotation::Final),
        (BuiltinAnnotation::Appendable, Annotation::Appendable),
        (BuiltinAnnotation::Mutable, Annotation::Mutable),
        (
            BuiltinAnnotation::Key {
                value: Some(PositiveIntConst(int_expr("4"))),
            },
            Annotation::Key {
                value: Some("4".to_string()),
            },
        ),
        (
            BuiltinAnnotation::MustUnderstand {
                value: Some(PositiveIntConst(int_expr("5"))),
            },
            Annotation::MustUnderstand {
                value: Some("5".to_string()),
            },
        ),
        (
            BuiltinAnnotation::DefaultLiteral,
            Annotation::DefaultLiteral,
        ),
        (
            BuiltinAnnotation::Default {
                value: string_expr("\"hello\""),
            },
            Annotation::Default {
                value: "\"hello\"".to_string(),
            },
        ),
        (
            BuiltinAnnotation::Range {
                min: PositiveIntConst(int_expr("1")),
                max: PositiveIntConst(int_expr("9")),
            },
            Annotation::Range {
                min: "1".to_string(),
                max: "9".to_string(),
            },
        ),
        (
            BuiltinAnnotation::Min {
                value: PositiveIntConst(int_expr("10")),
            },
            Annotation::Min {
                value: "10".to_string(),
            },
        ),
        (
            BuiltinAnnotation::Max {
                value: PositiveIntConst(int_expr("11")),
            },
            Annotation::Max {
                value: "11".to_string(),
            },
        ),
        (
            BuiltinAnnotation::Unit {
                value: string_expr("\"ms\""),
            },
            Annotation::Unit {
                value: "\"ms\"".to_string(),
            },
        ),
        (
            BuiltinAnnotation::BitBound {
                value: PositiveIntConst(int_expr("32")),
            },
            Annotation::BitBound {
                value: "32".to_string(),
            },
        ),
        (
            BuiltinAnnotation::External {
                value: Some(PositiveIntConst(int_expr("1"))),
            },
            Annotation::External {
                value: Some("1".to_string()),
            },
        ),
        (
            BuiltinAnnotation::Nested {
                value: Some(PositiveIntConst(int_expr("1"))),
            },
            Annotation::Nested {
                value: Some("1".to_string()),
            },
        ),
        (
            BuiltinAnnotation::Verbatim {
                language: Some(VerbatimLanguage::Cpp),
                placement: Some(PlacementKind::EndFile),
                text: string_expr("\"body\""),
            },
            Annotation::Verbatim {
                language: Some("c++".to_string()),
                placement: Some("END_FILE".to_string()),
                text: "\"body\"".to_string(),
            },
        ),
        (
            BuiltinAnnotation::Service {
                platform: Some(ServicePlatform::Dds),
            },
            Annotation::Service {
                platform: Some("DDS".to_string()),
            },
        ),
        (
            BuiltinAnnotation::Oneway {
                value: Some(scoped_expr(true, &["pkg", "Call"])),
            },
            Annotation::Oneway {
                value: Some("pkg::Call".to_string()),
            },
        ),
        (
            BuiltinAnnotation::Ami {
                value: Some(string_expr("\"ami\"")),
            },
            Annotation::Ami {
                value: Some("\"ami\"".to_string()),
            },
        ),
        (
            BuiltinAnnotation::HashId {
                value: Some(string_expr("\"hash\"")),
            },
            Annotation::HashId {
                value: Some("\"hash\"".to_string()),
            },
        ),
        (
            BuiltinAnnotation::DefaultNested {
                value: Some(bool_expr(false)),
            },
            Annotation::DefaultNested {
                value: Some("false".to_string()),
            },
        ),
        (
            BuiltinAnnotation::IgnoreLiteralNames {
                value: Some(bool_expr(true)),
            },
            Annotation::IgnoreLiteralNames {
                value: Some("true".to_string()),
            },
        ),
        (
            BuiltinAnnotation::TryConstruct {
                value: Some(TryConstructFailAction::UseDefault),
            },
            Annotation::TryConstruct {
                value: Some("USE_DEFAULT".to_string()),
            },
        ),
        (
            BuiltinAnnotation::NonSerialized {
                value: Some(BooleanLiteral::False),
            },
            Annotation::NonSerialized {
                value: Some("false".to_string()),
            },
        ),
        (
            BuiltinAnnotation::DataRepresentation {
                kinds: vec![DataRepresentationKind::Xcdr1, DataRepresentationKind::Xml],
            },
            Annotation::DataRepresentation {
                kinds: vec!["XCDR1".to_string(), "XML".to_string()],
            },
        ),
        (
            BuiltinAnnotation::Topic {
                name: Some(string_expr("\"orders\"")),
                platform: Some(TopicPlatform::Dds),
            },
            Annotation::Topic {
                name: Some("\"orders\"".to_string()),
                platform: Some("DDS".to_string()),
            },
        ),
        (BuiltinAnnotation::Choice, Annotation::Choice),
        (BuiltinAnnotation::Empty, Annotation::Empty),
        (BuiltinAnnotation::DdsService, Annotation::DdsService),
        (
            BuiltinAnnotation::DdsRequestTopic {
                name: string_expr("\"req\""),
            },
            Annotation::DdsRequestTopic {
                name: "\"req\"".to_string(),
            },
        ),
        (
            BuiltinAnnotation::DdsReplyTopic {
                name: string_expr("\"rep\""),
            },
            Annotation::DdsReplyTopic {
                name: "\"rep\"".to_string(),
            },
        ),
    ];

    for (input, expected) in cases {
        let actual = from_builtin_annotation(input).expect("annotation");
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}

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
        crate::hir::FloatingPtLiteral {
            sign: Some(IntegerSign::Minus),
            integer: crate::hir::DecNumber("12".to_string()),
            fraction: crate::hir::DecNumber("5".to_string()),
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
        render_hir_const_expr(&ConstExpr::ScopedName(crate::hir::ScopedName {
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

#[test]
fn annotation_from_builtin_path_uses_conversion_function() {
    let appl = AnnotationAppl {
        name: AnnotationName::Builtin("id".to_string()),
        params: None,
        builtin: Some(BuiltinAnnotation::Id {
            value: TypedIntegerLiteral::BinNumber("0b101".to_string()),
        }),
        is_extend: false,
        extra: Vec::new(),
    };
    assert_eq!(
        serde_json::to_value(Annotation::from(appl)).unwrap(),
        serde_json::json!({"Id":{"value":"0b101"}})
    );
}
