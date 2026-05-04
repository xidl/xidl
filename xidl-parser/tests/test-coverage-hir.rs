use xidl_parser::hir::{
    BitFieldType, ConstrTypeDcl, Definition, Specification, TypeDcl, TypeSpec, expand_annotations,
};
use xidl_parser::parser::parser_text;
use xidl_parser::typed_ast::{
    AnnotationAppl, AnnotationName, BaseTypeSpec, Identifier, IntegerType, ScopedName, SignedInt,
    SimpleTypeSpec as TypedSimpleTypeSpec, TemplateType, TemplateTypeSpec,
    TypeSpec as TypedTypeSpec,
};

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
fn hir_conversion_handles_union_field_ids_bitfields_and_typedef_shapes() {
    let typed = parser_text(
        r#"
        @id(7)
        bitset Flags {
            bitfield<1, boolean> ready active;
            bitfield<2, octet> raw;
            bitfield<3, int8> signed_value;
            bitfield<4, uint8> unsigned_value;
        };

        bitmask Permissions { @key read, write };

        union Choice switch (long) {
            case 0: @id(9) long same;
            case 1: long same;
            default: short other;
        };

        typedef sequence<long, 2> LongSeq, LongArray[4];
        native NativeThing;
        "#,
    )
    .expect("parse should succeed");
    let hir = Specification::from(typed);

    let Definition::TypeDcl(type_dcl) = &hir.0[0] else {
        panic!("expected bitset");
    };
    let TypeDcl::ConstrTypeDcl(ConstrTypeDcl::BitsetDcl(bitset)) = type_dcl else {
        panic!("expected bitset");
    };
    assert_eq!(bitset.field.len(), 5);
    assert!(matches!(bitset.field[0].ty, Some(BitFieldType::Bool)));
    assert!(matches!(bitset.field[1].ty, Some(BitFieldType::Bool)));
    assert!(matches!(bitset.field[2].ty, Some(BitFieldType::Octec)));
    assert!(matches!(bitset.field[3].ty, Some(BitFieldType::SignedInt)));
    assert!(matches!(
        bitset.field[4].ty,
        Some(BitFieldType::UnsignedInt)
    ));

    let Definition::TypeDcl(type_dcl) = &hir.0[2] else {
        panic!("expected union");
    };
    let TypeDcl::ConstrTypeDcl(ConstrTypeDcl::UnionDef(union)) = type_dcl else {
        panic!("expected union");
    };
    assert_eq!(
        union.case[0].element.field_id,
        union.case[1].element.field_id
    );
    assert_ne!(
        union.case[0].element.field_id,
        union.case[2].element.field_id
    );
    assert!(union.case[0].element.field_id.is_some());
    assert!(union.case[2].element.field_id.is_some());

    let Definition::TypeDcl(type_dcl) = &hir.0[3] else {
        panic!("expected typedef");
    };
    let TypeDcl::TypedefDcl(typedef) = type_dcl else {
        panic!("expected typedef");
    };
    assert_eq!(typedef.decl.len(), 2);

    let Definition::TypeDcl(type_dcl) = &hir.0[4] else {
        panic!("expected native");
    };
    assert!(matches!(type_dcl, TypeDcl::NativeDcl(_)));
}

#[test]
fn hir_type_conversions_cover_simple_and_template_variants() {
    let scoped_name = typed_scoped_name(&["demo", "Thing"], false);
    let template = TypedTypeSpec::TemplateTypeSpec(TemplateTypeSpec::TemplateType(TemplateType {
        ident: Identifier("Boxed".to_string()),
        args: vec![TypedTypeSpec::SimpleTypeSpec(
            TypedSimpleTypeSpec::ScopedName(scoped_name),
        )],
    }));
    let simple = TypedTypeSpec::SimpleTypeSpec(TypedSimpleTypeSpec::BaseTypeSpec(
        BaseTypeSpec::IntegerType(IntegerType::SignedInt(SignedInt::SignedLongLongInt(
            xidl_parser::typed_ast::SignedLongLongInt,
        ))),
    ));

    let template_hir: TypeSpec = template.into();
    let simple_hir: TypeSpec = simple.into();

    assert!(matches!(template_hir, TypeSpec::TemplateType(_)));
    assert!(matches!(
        simple_hir,
        TypeSpec::IntegerType(xidl_parser::hir::IntegerType::I64)
    ));

    let nested = AnnotationAppl {
        name: AnnotationName::Builtin("outer".to_string()),
        params: None,
        builtin: None,
        is_extend: false,
        extra: vec![AnnotationAppl::doc("inner".to_string())],
    };
    let expanded = expand_annotations(vec![nested]);
    assert_eq!(expanded.len(), 2);
}

#[test]
fn hir_struct_and_type_dcl_cover_optional_and_inline_typedef_paths() {
    let typed = parser_text(
        r#"
        @mutable
        struct Item {
            @optional long builtin_optional;
        };
        native NativeThing;
        "#,
    )
    .expect("parse should succeed");
    let hir = Specification::from(typed);

    let Definition::TypeDcl(struct_dcl) = &hir.0[0] else {
        panic!("expected struct");
    };
    let TypeDcl::ConstrTypeDcl(ConstrTypeDcl::StructDcl(item)) = struct_dcl else {
        panic!("expected struct def");
    };
    assert!(item.member[0].is_optional());

    let scoped_optional = xidl_parser::hir::Member {
        annotations: vec![xidl_parser::hir::Annotation::ScopedName {
            name: xidl_parser::hir::ScopedName {
                name: vec!["foo".to_string(), "optional".to_string()],
                is_root: false,
            },
            params: None,
        }],
        ty: xidl_parser::hir::TypeSpec::IntegerType(xidl_parser::hir::IntegerType::I32),
        ident: Vec::new(),
        default: Some(xidl_parser::hir::Default(
            xidl_parser::hir::ConstExpr::Literal(xidl_parser::hir::Literal::IntegerLiteral(
                xidl_parser::hir::IntegerLiteral("7".to_string()),
            )),
        )),
        field_id: None,
    };
    assert!(scoped_optional.is_optional());
    assert_eq!(
        xidl_parser::hir::const_expr_to_i64(&scoped_optional.default.unwrap().0),
        Some(7)
    );

    let typedef: xidl_parser::hir::TypedefDcl = xidl_parser::typed_ast::TypedefDcl {
        decl: xidl_parser::typed_ast::TypeDeclarator {
            ty: xidl_parser::typed_ast::TypeDeclaratorInner::ConstrTypeDcl(
                xidl_parser::typed_ast::ConstrTypeDcl::StructDcl(
                    xidl_parser::typed_ast::StructDcl::StructDef(
                        xidl_parser::typed_ast::StructDef {
                            ident: xidl_parser::typed_ast::Identifier("Inline".to_string()),
                            parent: Vec::new(),
                            member: Vec::new(),
                        },
                    ),
                ),
            ),
            decl: xidl_parser::typed_ast::AnyDeclarators(vec![
                xidl_parser::typed_ast::AnyDeclarator::SimpleDeclarator(
                    xidl_parser::typed_ast::SimpleDeclarator(xidl_parser::typed_ast::Identifier(
                        "InlineAlias".to_string(),
                    )),
                ),
            ]),
        },
    }
    .into();
    assert!(matches!(
        typedef.ty,
        xidl_parser::hir::TypedefType::ConstrTypeDcl(ConstrTypeDcl::StructDcl(_))
    ));

    let constr_inner: xidl_parser::hir::TypeDcl =
        xidl_parser::typed_ast::TypeDclInner::ConstrTypeDcl(
            xidl_parser::typed_ast::ConstrTypeDcl::EnumDcl(xidl_parser::typed_ast::EnumDcl {
                ident: xidl_parser::typed_ast::Identifier("Mode".to_string()),
                member: Vec::new(),
            }),
        )
        .into();
    assert!(matches!(constr_inner, TypeDcl::ConstrTypeDcl(_)));

    let typedef_inner: xidl_parser::hir::TypeDcl =
        xidl_parser::typed_ast::TypeDclInner::TypedefDcl(xidl_parser::typed_ast::TypedefDcl {
            decl: xidl_parser::typed_ast::TypeDeclarator {
                ty: xidl_parser::typed_ast::TypeDeclaratorInner::SimpleTypeSpec(
                    xidl_parser::typed_ast::SimpleTypeSpec::BaseTypeSpec(
                        xidl_parser::typed_ast::BaseTypeSpec::IntegerType(
                            xidl_parser::typed_ast::IntegerType::SignedInt(
                                xidl_parser::typed_ast::SignedInt::SignedLongInt(
                                    xidl_parser::typed_ast::SignedLongInt,
                                ),
                            ),
                        ),
                    ),
                ),
                decl: xidl_parser::typed_ast::AnyDeclarators(vec![
                    xidl_parser::typed_ast::AnyDeclarator::SimpleDeclarator(
                        xidl_parser::typed_ast::SimpleDeclarator(
                            xidl_parser::typed_ast::Identifier("Alias".to_string()),
                        ),
                    ),
                ]),
            },
        })
        .into();
    assert!(matches!(typedef_inner, TypeDcl::TypedefDcl(_)));

    let native_inner: xidl_parser::hir::TypeDcl =
        xidl_parser::typed_ast::TypeDclInner::NativeDcl(xidl_parser::typed_ast::NativeDcl {
            decl: xidl_parser::typed_ast::SimpleDeclarator(xidl_parser::typed_ast::Identifier(
                "NativeAlias".to_string(),
            )),
        })
        .into();
    assert!(matches!(native_inner, TypeDcl::NativeDcl(_)));

    let default_value: xidl_parser::hir::Default = xidl_parser::typed_ast::Default(
        xidl_parser::typed_ast::ConstExpr(xidl_parser::typed_ast::OrExpr::XorExpr(
            xidl_parser::typed_ast::XorExpr::AndExpr(xidl_parser::typed_ast::AndExpr::ShiftExpr(
                xidl_parser::typed_ast::ShiftExpr::AddExpr(
                    xidl_parser::typed_ast::AddExpr::MultExpr(
                        xidl_parser::typed_ast::MultExpr::UnaryExpr(
                            xidl_parser::typed_ast::UnaryExpr::PrimaryExpr(
                                xidl_parser::typed_ast::PrimaryExpr::Literal(
                                    xidl_parser::typed_ast::Literal::IntegerLiteral(
                                        xidl_parser::typed_ast::IntegerLiteral::DecNumber(
                                            "9".to_string(),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            )),
        )),
    )
    .into();
    assert_eq!(
        xidl_parser::hir::const_expr_to_i64(&default_value.0),
        Some(9)
    );

    let Definition::TypeDcl(native_dcl) = &hir.0[1] else {
        panic!("expected native");
    };
    assert!(matches!(native_dcl, TypeDcl::NativeDcl(_)));
}
