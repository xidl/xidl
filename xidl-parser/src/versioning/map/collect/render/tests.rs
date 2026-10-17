use super::*;

fn positive(value: &str) -> hir::PositiveIntConst {
    hir::PositiveIntConst(hir::ConstExpr::Literal(hir::Literal::IntegerLiteral(
        hir::IntegerLiteral(value.to_string()),
    )))
}

fn scoped(name: &str, is_root: bool) -> hir::ScopedName {
    hir::ScopedName {
        name: vec![name.to_string()],
        is_root,
    }
}

#[test]
fn renders_every_type_shape_without_dropping_contract_facts() {
    let integer_types = [
        (hir::IntegerType::Char, "char"),
        (hir::IntegerType::UChar, "uint8"),
        (hir::IntegerType::Octet, "uint8"),
        (hir::IntegerType::U8, "uint8"),
        (hir::IntegerType::U16, "unsigned short"),
        (hir::IntegerType::U32, "unsigned long"),
        (hir::IntegerType::U64, "unsigned long long"),
        (hir::IntegerType::I8, "int8"),
        (hir::IntegerType::I16, "short"),
        (hir::IntegerType::I32, "long"),
        (hir::IntegerType::I64, "long long"),
    ];
    for (integer, expected) in integer_types {
        assert_eq!(render_type(&hir::TypeSpec::IntegerType(integer)), expected);
    }

    assert_eq!(render_type(&hir::TypeSpec::FloatingPtType), "double");
    assert_eq!(render_type(&hir::TypeSpec::CharType), "char");
    assert_eq!(render_type(&hir::TypeSpec::WideCharType), "wchar");
    assert_eq!(render_type(&hir::TypeSpec::Boolean), "boolean");
    assert_eq!(render_type(&hir::TypeSpec::AnyType), "any");
    assert_eq!(render_type(&hir::TypeSpec::ObjectType), "Object");
    assert_eq!(render_type(&hir::TypeSpec::ValueBaseType), "ValueBase");
    assert_eq!(
        render_type(&hir::TypeSpec::ScopedName(scoped("Thing", true))),
        "::Thing"
    );
    assert_eq!(
        render_type(&hir::TypeSpec::SequenceType(hir::SequenceType {
            ty: Box::new(hir::TypeSpec::StringType(hir::StringType {
                bound: Some(positive("16")),
            })),
            len: Some(positive("4")),
        })),
        "sequence<string<16>, 4>"
    );
    assert_eq!(
        render_type(&hir::TypeSpec::WideStringType(hir::WideStringType {
            bound: Some(positive("32")),
        })),
        "wstring<32>"
    );
    assert_eq!(
        render_type(&hir::TypeSpec::FixedPtType(hir::FixedPtType {
            integer: positive("8"),
            fraction: positive("2"),
        })),
        "fixed<8, 2>"
    );
    assert_eq!(
        render_type(&hir::TypeSpec::MapType(hir::MapType {
            key: Box::new(hir::TypeSpec::StringType(hir::StringType { bound: None })),
            value: Box::new(hir::TypeSpec::IntegerType(hir::IntegerType::U32)),
            len: Some(positive("8")),
        })),
        "map<string, unsigned long, 8>"
    );
    assert_eq!(
        render_type(&hir::TypeSpec::TemplateType(hir::TemplateType {
            ident: "Box".to_string(),
            args: vec![hir::TypeSpec::Boolean],
        })),
        "Box<boolean>"
    );
}

#[test]
fn renders_declarator_shapes_and_constant_expressions() {
    let simple = hir::Declarator::SimpleDeclarator(hir::SimpleDeclarator("value".to_string()));
    assert_eq!(declarator_ident(&simple), "value");
    assert_eq!(declarator_facts(&simple)["name"], "value");

    let array = hir::Declarator::ArrayDeclarator(hir::ArrayDeclarator {
        ident: "items".to_string(),
        len: vec![positive("3"), positive("4")],
    });
    assert_eq!(declarator_ident(&array), "items");
    assert_eq!(declarator_facts(&array)["array"], true);
    assert_eq!(declarator_facts(&array)["lengths"][0], "3");

    let expression = hir::ConstExpr::BinaryExpr(
        hir::BinaryOperator::Add,
        Box::new(hir::ConstExpr::UnaryExpr(
            hir::UnaryOperator::Sub,
            Box::new(hir::ConstExpr::Literal(hir::Literal::IntegerLiteral(
                hir::IntegerLiteral("1".to_string()),
            ))),
        )),
        Box::new(hir::ConstExpr::ScopedName(scoped("Limit", false))),
    );
    assert_eq!(render_const_expr(&expression), "(-1 + Limit)");
}
