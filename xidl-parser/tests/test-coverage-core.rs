use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use xidl_parser::hir::{
    Annotation, AnnotationParams, Definition, Extensibility, Pragma, SerializeConfig,
    SerializeKind, SerializeVersion, Specification, annotation_id_value, const_expr_to_i64,
    extensibility_from_annotations,
};
use xidl_parser::parser::{normalize_source_for_tree_sitter, parser_text};
use xidl_parser::typed_ast::{
    AddExpr, AndExpr, AnnotationName, ConstExpr, DecNumber, FloatingPtLiteral, Identifier,
    IntegerLiteral, IntegerSign, Literal, MultExpr, OrExpr, PrimaryExpr, ScopedName, ShiftExpr,
    UnaryExpr, UnaryOperator, XorExpr,
};

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "xidl-parser-coverage-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_file(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, source).expect("write fixture");
}

fn parse_hir_with_path(path: &Path) -> xidl_parser::error::ParserResult<Specification> {
    let source = fs::read_to_string(path).expect("read fixture");
    let typed = parser_text(&source)?;
    Specification::from_typed_ast_with_path(typed, path)
}

fn int_expr(literal: IntegerLiteral) -> ConstExpr {
    ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::PrimaryExpr(PrimaryExpr::Literal(Literal::IntegerLiteral(literal))),
        ))),
    ))))
}

fn typed_scoped_name(parts: &[&str], is_root: bool) -> ScopedName {
    let mut current = None;
    for part in parts {
        current = Some(ScopedName {
            scoped_name: current.map(Box::new),
            identifier: Identifier((*part).to_string()),
            node_text: String::new(),
        });
    }
    let mut scoped = current.expect("at least one part");
    scoped.node_text = format!("{}{}", if is_root { "::" } else { "" }, parts.join("::"));
    scoped
}

#[test]
fn normalize_annotations_with_brackets_but_keep_strings() {
    let source = r#"@verbatim(language="rust", text=["a", "b"]) const string S = "@keep([1])";"#;
    let normalized = normalize_source_for_tree_sitter(source);

    assert!(normalized.contains("@verbatim"));
    assert!(normalized.contains("\"@keep([1])\""));
    assert!(!normalized.contains("[\"a\", \"b\"]"));
}

#[test]
fn parser_collects_doc_comments_and_complex_template_types() {
    let typed = parser_text(
        r#"
        /// Account balance
        typedef fixed<12, 4> Money;
        typedef map<long, string<8>, 16> Dict;
        typedef Vec<long, string> MyVec;
        "#,
    )
    .expect("parse should succeed");

    let xidl_parser::typed_ast::Specification(defs) = typed;
    let xidl_parser::typed_ast::Definition::TypeDcl(first) = &defs[0] else {
        panic!("expected typedef");
    };
    assert!(matches!(
        first.annotations[0].name,
        AnnotationName::Builtin(ref name) if name == "doc"
    ));

    let xidl_parser::typed_ast::Definition::TypeDcl(second) = &defs[1] else {
        panic!("expected typedef");
    };
    let xidl_parser::typed_ast::TypeDclInner::TypedefDcl(typedef) = &second.decl else {
        panic!("expected typedef decl");
    };
    assert!(matches!(
        typedef.decl.ty,
        xidl_parser::typed_ast::TypeDeclaratorInner::TemplateTypeSpec(
            xidl_parser::typed_ast::TemplateTypeSpec::MapType(_)
        )
    ));

    let xidl_parser::typed_ast::Definition::TypeDcl(third) = &defs[2] else {
        panic!("expected typedef");
    };
    let xidl_parser::typed_ast::TypeDclInner::TypedefDcl(typedef) = &third.decl else {
        panic!("expected typedef decl");
    };
    let xidl_parser::typed_ast::TypeDeclaratorInner::TemplateTypeSpec(
        xidl_parser::typed_ast::TemplateTypeSpec::TemplateType(template),
    ) = &typedef.decl.ty
    else {
        panic!("expected template type");
    };
    assert_eq!(template.ident.0, "Vec");
    assert_eq!(template.args.len(), 2);
}

#[test]
fn hir_parses_pragmas_and_include_errors() {
    let root = unique_temp_dir("pragma");
    let pragma = root.join("pragma.idl");
    let system_include = root.join("system_include.idl");
    let identifier_include = root.join("identifier_include.idl");

    write_file(
        &pragma,
        r#"
        #pragma xidlc XCDR2
        #pragma xidlc serialize(PL_CDR)
        #pragma xidlc package "demo.pkg"
        #pragma xidlc openapi version "1.2.3"
        #pragma xidlc service "https://example.test" "Demo service"
        "#,
    );
    write_file(&system_include, "#include <dds/core.idl>\n");
    write_file(&identifier_include, "#include SOME_HEADER\n");

    let hir = parse_hir_with_path(&pragma).expect("pragma parse should succeed");
    assert!(matches!(
        hir.0[0],
        Definition::Pragma(Pragma::XidlcVersion(SerializeVersion::Xcdr2))
    ));
    assert!(matches!(
        hir.0[1],
        Definition::Pragma(Pragma::XidlcSerialize(SerializeKind::PlCdr))
    ));
    assert!(matches!(
        hir.0[2],
        Definition::Pragma(Pragma::XidlcPackage(ref value)) if value == "demo.pkg"
    ));
    assert!(matches!(
        hir.0[3],
        Definition::Pragma(Pragma::XidlcOpenApiVersion(ref value)) if value == "1.2.3"
    ));
    assert!(matches!(
        hir.0[4],
        Definition::Pragma(Pragma::XidlcOpenApiService { ref base_url, ref description })
            if base_url == "https://example.test" && description.as_deref() == Some("Demo service")
    ));

    let err = parse_hir_with_path(&system_include).expect_err("system include must fail");
    assert!(err.to_string().contains("unsupported include path syntax"));

    let err = parse_hir_with_path(&identifier_include).expect_err("identifier include must fail");
    assert!(err.to_string().contains("unsupported include identifier"));
}

#[test]
fn serialize_config_and_expr_helpers_cover_resolution_branches() {
    let annotations = vec![
        Annotation::Builtin {
            name: "appendable".to_string(),
            params: None,
        },
        Annotation::Builtin {
            name: "extensibility".to_string(),
            params: Some(AnnotationParams::Raw("\"mutable\"".to_string())),
        },
        Annotation::Id {
            value: int_expr(IntegerLiteral::DecNumber("42".to_string())).into(),
        },
    ];

    assert_eq!(annotation_id_value(&annotations), Some(42));
    assert!(matches!(
        extensibility_from_annotations(&annotations),
        Extensibility::Mutable
    ));

    let mut config = SerializeConfig::default();
    assert!(matches!(
        config.resolve(Extensibility::None),
        SerializeKind::Cdr
    ));
    config.apply_pragma(Pragma::XidlcVersion(SerializeVersion::Xcdr1));
    assert!(matches!(
        config.resolve(Extensibility::Mutable),
        SerializeKind::PlCdr
    ));
    assert!(matches!(
        config.resolve(Extensibility::Appendable),
        SerializeKind::Cdr
    ));
    assert!(matches!(
        config.resolve(Extensibility::None),
        SerializeKind::PlainCdr
    ));
    config.apply_pragma(Pragma::XidlcSerialize(SerializeKind::DelimitedCdr));
    assert!(matches!(
        config.resolve_for_annotations(&annotations),
        SerializeKind::DelimitedCdr
    ));

    assert_eq!(
        const_expr_to_i64(&int_expr(IntegerLiteral::BinNumber("0b1_010".to_string())).into()),
        Some(10)
    );
    assert_eq!(
        const_expr_to_i64(&int_expr(IntegerLiteral::OctNumber("0O17".to_string())).into()),
        Some(15)
    );
    assert_eq!(
        const_expr_to_i64(&int_expr(IntegerLiteral::HexNumber("0x1f".to_string())).into()),
        Some(31)
    );

    let negative = ConstExpr(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
        ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
            UnaryExpr::UnaryExpr(
                UnaryOperator::Sub,
                PrimaryExpr::Literal(Literal::IntegerLiteral(IntegerLiteral::DecNumber(
                    "7".to_string(),
                ))),
            ),
        ))),
    ))));
    assert_eq!(const_expr_to_i64(&negative.into()), Some(-7));

    let unsupported = ConstExpr(OrExpr::OrExpr(
        Box::new(OrExpr::XorExpr(XorExpr::AndExpr(AndExpr::ShiftExpr(
            ShiftExpr::AddExpr(AddExpr::MultExpr(MultExpr::UnaryExpr(
                UnaryExpr::PrimaryExpr(PrimaryExpr::ScopedName(typed_scoped_name(
                    &["Demo", "VALUE"],
                    true,
                ))),
            ))),
        )))),
        XorExpr::AndExpr(AndExpr::ShiftExpr(ShiftExpr::AddExpr(AddExpr::MultExpr(
            MultExpr::UnaryExpr(UnaryExpr::UnaryExpr(
                UnaryOperator::Not,
                PrimaryExpr::ConstExpr(Box::new(int_expr(IntegerLiteral::DecNumber(
                    "1".to_string(),
                )))),
            )),
        )))),
    ));
    assert_eq!(const_expr_to_i64(&unsupported.into()), None);

    let custom = Specification::from_typed_ast_with_properties(
        parser_text("interface Example { void ping(); };").expect("parse"),
        [("expand_interface".to_string(), json!(false))]
            .into_iter()
            .collect(),
    );
    assert_eq!(custom.0.len(), 1);

    let float: xidl_parser::hir::Literal = Literal::FloatingPtLiteral(FloatingPtLiteral {
        sign: Some(IntegerSign::Minus),
        integer: DecNumber("12".to_string()),
        fraction: DecNumber("5".to_string()),
    })
    .into();
    assert!(matches!(
        float,
        xidl_parser::hir::Literal::FloatingPtLiteral(_)
    ));
}
