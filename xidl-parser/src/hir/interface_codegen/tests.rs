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

fn int(value: &str) -> ConstExpr {
    ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(value.to_string())))
}

fn scoped_expr(parts: &[&str], is_root: bool) -> ConstExpr {
    ConstExpr::ScopedName(scoped(parts, is_root))
}

fn wrap_unary(op: UnaryOperator, value: ConstExpr) -> ConstExpr {
    ConstExpr::UnaryExpr(op, Box::new(value))
}

fn literal(value: Literal) -> ConstExpr {
    ConstExpr::Literal(value)
}

fn nested(value: ConstExpr) -> ConstExpr {
    ConstExpr::UnaryExpr(UnaryOperator::Add, Box::new(value))
}

fn binary(op: BinaryOperator, left: ConstExpr, right: ConstExpr) -> ConstExpr {
    ConstExpr::BinaryExpr(op, Box::new(left), Box::new(right))
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
        render_const_expr(&binary(BinaryOperator::Or, int("1"), int("2"))),
        render_const_expr(&binary(BinaryOperator::Xor, int("1"), int("2"))),
        render_const_expr(&binary(BinaryOperator::And, int("1"), int("2"))),
        render_const_expr(&binary(BinaryOperator::LeftShift, int("1"), int("2"))),
        render_const_expr(&binary(BinaryOperator::RightShift, int("4"), int("1"))),
        render_const_expr(&binary(BinaryOperator::Add, int("1"), int("2"))),
        render_const_expr(&binary(BinaryOperator::Sub, int("3"), int("1"))),
        render_const_expr(&binary(BinaryOperator::Mult, int("2"), int("3"))),
        render_const_expr(&binary(BinaryOperator::Div, int("6"), int("2"))),
        render_const_expr(&binary(BinaryOperator::Mod, int("7"), int("3"))),
        render_const_expr(&wrap_unary(UnaryOperator::Not, int("1"))),
        render_const_expr(&scoped_expr(&["demo", "VALUE"], true)),
        render_const_expr(&nested(int("8"))),
        render_const_expr(&literal(Literal::FloatingPtLiteral(FloatingPtLiteral {
            sign: Some(IntegerSign("-".to_string())),
            integer: DecNumber("1".to_string()),
            fraction: DecNumber("5".to_string()),
        }))),
        render_const_expr(&literal(Literal::CharLiteral("'a'".to_string()))),
        render_const_expr(&literal(Literal::WideCharacterLiteral("L'a'".to_string()))),
        render_const_expr(&literal(Literal::StringLiteral("\"hi\"".to_string()))),
        render_const_expr(&literal(Literal::WideStringLiteral("L\"hi\"".to_string()))),
        render_const_expr(&literal(Literal::BooleanLiteral("TRUE".to_string()))),
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
