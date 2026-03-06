use xidl_parser::hir;

fn first_struct(spec: &hir::Specification) -> &hir::StructDcl {
    for def in &spec.0 {
        if let hir::Definition::TypeDcl(type_dcl) = def
            && let hir::TypeDclInner::ConstrTypeDcl(hir::ConstrTypeDcl::StructDcl(def)) =
                &type_dcl.decl
        {
            return def;
        }
    }
    panic!("expected first struct definition");
}

#[test]
fn test_struct_optional_field_uses_plcdr() {
    let typed = xidl_parser::parser::parser_text(
        r#"
        struct S {
            @optional long a;
            long b;
        };
        "#,
    )
    .unwrap();
    let spec = hir::Specification::from(typed);
    let def = first_struct(&spec);

    let cfg1 = hir::SerializeConfig {
        explicit_kind: None,
        version: Some(hir::SerializeVersion::Xcdr1),
    };
    assert!(matches!(
        def.serialize_kind(&cfg1),
        hir::SerializeKind::PlCdr
    ));

    let cfg2 = hir::SerializeConfig {
        explicit_kind: None,
        version: Some(hir::SerializeVersion::Xcdr2),
    };
    assert!(matches!(
        def.serialize_kind(&cfg2),
        hir::SerializeKind::PlCdr2
    ));
}
