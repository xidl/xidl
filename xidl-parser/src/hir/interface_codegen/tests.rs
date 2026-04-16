use super::context::{ConstContext, TemplateContext};
use super::render::render_template;
use super::types::{
    idl_type_spec, qualified_exception_name, render_const_expr, scoped_name_to_idl,
};
use super::*;

fn scoped(parts: &[&str], is_root: bool) -> ScopedName {
    ScopedName {
        name: parts.iter().map(|part| (*part).to_string()).collect(),
        is_root,
    }
}

fn int(value: &str) -> PrimaryExpr {
    PrimaryExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(value.to_string())))
}

fn wrap_primary(value: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(value),
        ))),
    ))))
}

fn wrap_unary(op: UnaryOperator, value: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::UnaryExpr(op, value),
        ))),
    ))))
}

fn or_expr(left: ConstExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::OrExpr(
        Box::new(left.0),
        XorExpr::AndExpr(AndExpr::ShiftExpr(ShiftExpr::AddExpr(AddExpr::MultExpr(
            MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(right)),
        )))),
    ))
}

fn xor_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::XorExpr(
        Box::new(XorExpr::AndExpr(AndExpr::ShiftExpr(ShiftExpr::AddExpr(
            AddExpr::MultExpr(MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(left))),
        )))),
        AndExpr::ShiftExpr(ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(right),
        )))),
    )))
}

fn and_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::AndExpr(
        Box::new(AndExpr::ShiftExpr(ShiftExpr::AddExpr(AddExpr::MultExpr(
            MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(left)),
        )))),
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(right),
        ))),
    ))))
}

fn lshift_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::LeftShiftExpr(
            Box::new(ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
                UnaryExpr::PrimaryExpr(left),
            )))),
            AddExpr::MultExpr(MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(right))),
        ),
    ))))
}

fn rshift_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::RightShiftExpr(
            Box::new(ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
                UnaryExpr::PrimaryExpr(left),
            )))),
            AddExpr::MultExpr(MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(right))),
        ),
    ))))
}

fn add_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::AddExpr(
            Box::new(AddExpr::MultExpr(MultExpr::UnaryExpr(
                UnaryExpr::PrimaryExpr(left),
            ))),
            MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(right)),
        )),
    ))))
}

fn sub_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::SubExpr(
            Box::new(AddExpr::MultExpr(MultExpr::UnaryExpr(
                UnaryExpr::PrimaryExpr(left),
            ))),
            MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(right)),
        )),
    ))))
}

fn mult_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::MultExpr(
            Box::new(MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(left))),
            UnaryExpr::PrimaryExpr(right),
        ))),
    ))))
}

fn div_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::DivExpr(
            Box::new(MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(left))),
            UnaryExpr::PrimaryExpr(right),
        ))),
    ))))
}

fn mod_expr(left: PrimaryExpr, right: PrimaryExpr) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::ModExpr(
            Box::new(MultExpr::UnaryExpr(UnaryExpr::PrimaryExpr(left))),
            UnaryExpr::PrimaryExpr(right),
        ))),
    ))))
}

fn parse_interface(source: &str) -> InterfaceDcl {
    let typed = crate::parser::parser_text(source).expect("parse should succeed");
    let spec = crate::hir::Specification::from_typed_ast_with_properties(
        typed,
        [("expand_interface".to_string(), serde_json::json!(false))]
            .into_iter()
            .collect(),
    );
    match spec.0.into_iter().next().expect("interface") {
        Definition::InterfaceDcl(value) => value,
        other => panic!("expected interface, got {other:?}"),
    }
}

#[test]
fn renders_idl_types_and_expressions() {
    let rendered = [
        render_const_expr(&or_expr(wrap_primary(int("1")), int("2"))),
        render_const_expr(&xor_expr(int("1"), int("2"))),
        render_const_expr(&and_expr(int("1"), int("2"))),
        render_const_expr(&lshift_expr(int("1"), int("2"))),
        render_const_expr(&rshift_expr(int("4"), int("1"))),
        render_const_expr(&add_expr(int("1"), int("2"))),
        render_const_expr(&sub_expr(int("3"), int("1"))),
        render_const_expr(&mult_expr(int("2"), int("3"))),
        render_const_expr(&div_expr(int("6"), int("2"))),
        render_const_expr(&mod_expr(int("7"), int("3"))),
        render_const_expr(&wrap_unary(UnaryOperator::Not, int("1"))),
        render_const_expr(&wrap_primary(PrimaryExpr::ScopedName(scoped(
            &["demo", "VALUE"],
            true,
        )))),
        render_const_expr(&wrap_primary(PrimaryExpr::ConstExpr(Box::new(
            wrap_primary(int("8")),
        )))),
        render_const_expr(&wrap_primary(PrimaryExpr::Literal(
            Literal::FloatingPtLiteral(FloatingPtLiteral {
                sign: Some(IntegerSign("-".to_string())),
                integer: DecNumber("1".to_string()),
                fraction: DecNumber("5".to_string()),
            }),
        ))),
        render_const_expr(&wrap_primary(PrimaryExpr::Literal(Literal::CharLiteral(
            "'a'".to_string(),
        )))),
        render_const_expr(&wrap_primary(PrimaryExpr::Literal(
            Literal::WideCharacterLiteral("L'a'".to_string()),
        ))),
        render_const_expr(&wrap_primary(PrimaryExpr::Literal(Literal::StringLiteral(
            "\"hi\"".to_string(),
        )))),
        render_const_expr(&wrap_primary(PrimaryExpr::Literal(
            Literal::WideStringLiteral("L\"hi\"".to_string()),
        ))),
        render_const_expr(&wrap_primary(PrimaryExpr::Literal(
            Literal::BooleanLiteral("TRUE".to_string()),
        ))),
    ];

    for token in ["|", "^", "&", "<<", ">>", "+", "-", "*", "/", "%", "~"] {
        assert!(
            rendered.iter().any(|value| value.contains(token)),
            "{token}"
        );
    }
    for value in [
        "'a'",
        "L'a'",
        "\"hi\"",
        "L\"hi\"",
        "true",
        "-1.5",
        "::demo::VALUE",
    ] {
        assert!(rendered.iter().any(|item| item == value), "{value}");
    }

    let rendered_types = [
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::IntegerType(IntegerType::Char)),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::IntegerType(IntegerType::UChar)),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::FloatingPtType),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::CharType),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::WideCharType),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::Boolean),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::AnyType),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::ObjectType),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::ValueBaseType),
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::ScopedName(scoped(&["demo", "Thing"], true))),
        TypeSpec::TemplateTypeSpec(TemplateTypeSpec::WideStringType(WideStringType {
            bound: None,
        })),
        TypeSpec::TemplateTypeSpec(TemplateTypeSpec::MapType(MapType {
            key: Box::new(TypeSpec::SimpleTypeSpec(SimpleTypeSpec::CharType)),
            value: Box::new(TypeSpec::SimpleTypeSpec(SimpleTypeSpec::Boolean)),
            len: None,
        })),
        TypeSpec::TemplateTypeSpec(TemplateTypeSpec::TemplateType(TemplateType {
            ident: "Box".to_string(),
            args: vec![
                TypeSpec::SimpleTypeSpec(SimpleTypeSpec::IntegerType(IntegerType::I32)),
                TypeSpec::SimpleTypeSpec(SimpleTypeSpec::CharType),
            ],
        })),
    ]
    .iter()
    .map(idl_type_spec)
    .collect::<Vec<_>>();

    for value in [
        "double",
        "int8",
        "uint8",
        "char",
        "wchar",
        "boolean",
        "any",
        "Object",
        "ValueBase",
        "::demo::Thing",
        "wstring",
        "map<char, boolean>",
        "Box<long, char>",
    ] {
        assert!(rendered_types.iter().any(|item| item == value), "{value}");
    }
    assert_eq!(scoped_name_to_idl(&scoped(&["Problem"], false)), "Problem");
    assert_eq!(
        qualified_exception_name(&scoped(&["Problem"], false), &["demo".to_string()]),
        "demo::Problem"
    );
    assert_eq!(
        qualified_exception_name(&scoped(&["Root", "Problem"], true), &["demo".to_string()]),
        "Root::Problem"
    );
}

#[test]
fn collects_operations_and_expands_interface() {
    let interface = parse_interface(
        r#"
        interface Service {
            readonly attribute boolean ready;
            readonly attribute boolean check raises(ReadError);
            attribute long value getraises(GetError) setraises(SetError);
            attribute long return_ setraises(SetOnly);
            void ping();
            long compute(in long input, out long return_) raises(Failure, ::RootFailure);
        };
        "#,
    );

    let InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        panic!("expected interface def");
    };
    let body = def.interface_body.as_ref().expect("body");
    let ops = collect_operations(body);
    assert!(ops.iter().any(|op| op.name == "get_attribute_ready"));
    assert!(ops.iter().any(|op| op.name == "set_attribute_value"));
    assert!(ops.iter().any(|op| op.name == "compute"));

    let compute = ops.iter().find(|op| op.name == "compute").expect("compute");
    let ctx = compute.to_context();
    assert!(
        ctx.out_members
            .iter()
            .any(|member| member.name == "return_1")
    );
    assert!(
        ctx.result_exceptions
            .iter()
            .any(|value| value.ty == "::RootFailure")
    );

    let expanded = expand_interface(&interface, &["demo".to_string()]).expect("expand");
    assert!(!expanded.is_empty());
}

#[test]
fn renders_templates_and_reports_missing_files() {
    let ctx = TemplateContext {
        modules: vec!["demo".to_string()],
        interface_name: "Service".to_string(),
        consts: vec![ConstContext {
            name: "SERVICE_PING_HASH".to_string(),
            value: "1".to_string(),
        }],
        operations: Vec::new(),
    };

    let rendered = render_template("interface.idl.j2", &ctx).expect("template");
    assert!(rendered.contains("module demo"));

    let err = render_template("missing.idl.j2", &ctx).expect_err("missing template must fail");
    assert!(err.to_string().contains("missing template"));
}
