use super::*;

#[test]
fn render_helpers_cover_types_expressions_and_declarators() {
    let integer_types = [
        (IntegerType::Char, "char"),
        (IntegerType::UChar, "uint8"),
        (IntegerType::Octet, "octet"),
        (IntegerType::U8, "uint8"),
        (IntegerType::U16, "unsigned short"),
        (IntegerType::U32, "unsigned long"),
        (IntegerType::U64, "unsigned long long"),
        (IntegerType::I8, "int8"),
        (IntegerType::I16, "short"),
        (IntegerType::I32, "long"),
        (IntegerType::I64, "long long"),
    ];
    for (value, expected) in integer_types {
        assert_eq!(IdlPrinter::render_integer_type(&value), expected);
    }

    let types = [
        TypeSpec::IntegerType(IntegerType::I32),
        TypeSpec::FloatingPtType,
        TypeSpec::CharType,
        TypeSpec::WideCharType,
        TypeSpec::Boolean,
        TypeSpec::AnyType,
        TypeSpec::ObjectType,
        TypeSpec::ValueBaseType,
        TypeSpec::ScopedName(scoped(&["pkg", "Item"], true)),
        TypeSpec::SequenceType(SequenceType {
            ty: Box::new(TypeSpec::StringType(StringType {
                bound: Some(positive("8")),
            })),
            len: Some(positive("4")),
        }),
        TypeSpec::StringType(StringType {
            bound: Some(positive("16")),
        }),
        TypeSpec::WideStringType(WideStringType {
            bound: Some(positive("16")),
        }),
        TypeSpec::FixedPtType(FixedPtType {
            integer: positive("10"),
            fraction: positive("2"),
        }),
        TypeSpec::MapType(MapType {
            key: Box::new(TypeSpec::CharType),
            value: Box::new(TypeSpec::Boolean),
            len: Some(positive("3")),
        }),
        TypeSpec::TemplateType(TemplateType {
            ident: "Pair".to_string(),
            args: vec![TypeSpec::IntegerType(IntegerType::I16), TypeSpec::CharType],
        }),
    ];
    let rendered = types
        .iter()
        .map(IdlPrinter::render_type)
        .collect::<Vec<_>>();
    for expected in [
        "long",
        "double",
        "char",
        "wchar",
        "boolean",
        "any",
        "Object",
        "ValueBase",
        "::pkg::Item",
        "sequence<string<8>, 4>",
        "string<16>",
        "wstring<16>",
        "fixed<10, 2>",
        "map<char, boolean, 3>",
        "Pair<short, char>",
    ] {
        assert!(rendered.iter().any(|value| value == expected), "{expected}");
    }
    assert_eq!(IdlPrinter::render_bounded_type("string", None), "string");

    let const_types = [
        ConstType::IntegerType(IntegerType::I32),
        ConstType::FloatingPtType,
        ConstType::FixedPtConstType,
        ConstType::CharType,
        ConstType::WideCharType,
        ConstType::BooleanType,
        ConstType::OctetType,
        ConstType::StringType(StringType { bound: None }),
        ConstType::WideStringType(WideStringType {
            bound: Some(positive("4")),
        }),
        ConstType::ScopedName(scoped(&["Thing"], false)),
        ConstType::SequenceType(SequenceType {
            ty: Box::new(TypeSpec::IntegerType(IntegerType::U8)),
            len: None,
        }),
    ];
    let const_rendered = const_types
        .iter()
        .map(IdlPrinter::render_const_type)
        .collect::<Vec<_>>();
    for expected in [
        "long",
        "double",
        "fixed",
        "char",
        "wchar",
        "boolean",
        "octet",
        "string",
        "wstring<4>",
        "Thing",
        "sequence<uint8>",
    ] {
        assert!(
            const_rendered.iter().any(|value| value == expected),
            "{expected}"
        );
    }

    let switch_types = [
        SwitchTypeSpec::IntegerType(IntegerType::I16),
        SwitchTypeSpec::CharType,
        SwitchTypeSpec::WideCharType,
        SwitchTypeSpec::BooleanType,
        SwitchTypeSpec::ScopedName(scoped(&["Kind"], false)),
        SwitchTypeSpec::OctetType,
    ];
    for expected in ["short", "char", "wchar", "boolean", "Kind", "octet"] {
        assert!(
            switch_types
                .iter()
                .map(IdlPrinter::render_switch_type)
                .any(|value| value == expected)
        );
    }
    for (value, expected) in [
        (BitFieldType::Bool, "boolean"),
        (BitFieldType::Octec, "octet"),
        (BitFieldType::SignedInt, "short"),
        (BitFieldType::UnsignedInt, "unsigned short"),
    ] {
        assert_eq!(IdlPrinter::render_bitfield_type(&value), expected);
    }

    let array = Declarator::ArrayDeclarator(ArrayDeclarator {
        ident: "matrix".to_string(),
        len: vec![positive("2"), positive("3")],
    });
    assert_eq!(IdlPrinter::render_declarator(&array), "matrix[2][3]");
    assert_eq!(
        IdlPrinter::render_declarator(&Declarator::SimpleDeclarator(SimpleDeclarator(
            "item".to_string(),
        ))),
        "item"
    );
    let exceptions = vec![scoped(&["Error"], false), scoped(&["pkg", "Failure"], true)];
    assert_eq!(
        IdlPrinter::render_exception_names(&exceptions),
        "Error, ::pkg::Failure"
    );
    assert_eq!(
        IdlPrinter::render_scoped(&scoped(&["pkg", "Thing"], true)),
        "::pkg::Thing"
    );

    let expression = ConstExpr::BinaryExpr(
        BinaryOperator::Mod,
        Box::new(ConstExpr::UnaryExpr(
            UnaryOperator::Sub,
            Box::new(integer("4")),
        )),
        Box::new(ConstExpr::ScopedName(scoped(&["Flags", "Read"], false))),
    );
    assert_eq!(IdlPrinter::render_expr(&expression), "(-4 % Flags::Read)");
    for (operator, expected) in [
        (UnaryOperator::Add, "+"),
        (UnaryOperator::Sub, "-"),
        (UnaryOperator::Not, "~"),
    ] {
        assert_eq!(IdlPrinter::render_unary_operator(&operator), expected);
    }
    for (operator, expected) in [
        (BinaryOperator::Or, "|"),
        (BinaryOperator::Xor, "^"),
        (BinaryOperator::And, "&"),
        (BinaryOperator::LeftShift, "<<"),
        (BinaryOperator::RightShift, ">>"),
        (BinaryOperator::Add, "+"),
        (BinaryOperator::Sub, "-"),
        (BinaryOperator::Mult, "*"),
        (BinaryOperator::Div, "/"),
        (BinaryOperator::Mod, "%"),
    ] {
        assert_eq!(IdlPrinter::render_binary_operator(&operator), expected);
    }
    for (literal, expected) in [
        (
            Literal::IntegerLiteral(IntegerLiteral("42".to_string())),
            "42".to_string(),
        ),
        (
            Literal::FloatingPtLiteral(FloatingPtLiteral {
                sign: Some(IntegerSign::Minus),
                integer: DecNumber("1".to_string()),
                fraction: DecNumber("25".to_string()),
            }),
            "-1.25".to_string(),
        ),
        (Literal::CharLiteral("c".to_string()), "'c'".to_string()),
        (
            Literal::WideCharacterLiteral("L'w'".to_string()),
            "L'w'".to_string(),
        ),
        (
            Literal::StringLiteral("hello".to_string()),
            "\"hello\"".to_string(),
        ),
        (
            Literal::WideStringLiteral("L\"wide\"".to_string()),
            "L\"wide\"".to_string(),
        ),
        (Literal::BooleanLiteral(true), "true".to_string()),
    ] {
        assert_eq!(IdlPrinter::render_literal(&literal), expected);
    }
    assert_eq!(IdlPrinter::quoted_literal("a'b", '\'', false), "'a\\'b'");
    assert_eq!(IdlPrinter::quoted_literal("wide", '"', true), "L\"wide\"");
}
