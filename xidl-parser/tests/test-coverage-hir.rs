use xidl_parser::hir::{
    expand_annotations, BitFieldType, ConstrTypeDcl, Definition, SimpleTypeSpec, Specification,
    TypeDclInner, TypeSpec,
};
use xidl_parser::parser::parser_text;
use xidl_parser::typed_ast::{
    AnnotationAppl, AnnotationName, BaseTypeSpec, Identifier, IntegerType, ScopedName, SignedInt,
    SimpleTypeSpec as TypedSimpleTypeSpec, TemplateType, TemplateTypeSpec, TypeSpec as TypedTypeSpec,
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
    scoped.node_text = format!(
        "{}{}",
        if is_root { "::" } else { "" },
        parts.join("::")
    );
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

    let Definition::TypeDcl(type_dcl) = &hir.0[0] else { panic!("expected bitset"); };
    let TypeDclInner::ConstrTypeDcl(ConstrTypeDcl::BitsetDcl(bitset)) = &type_dcl.decl else {
        panic!("expected bitset");
    };
    assert_eq!(bitset.field.len(), 5);
    assert!(matches!(bitset.field[0].ty, Some(BitFieldType::Bool)));
    assert!(matches!(bitset.field[1].ty, Some(BitFieldType::Bool)));
    assert!(matches!(bitset.field[2].ty, Some(BitFieldType::Octec)));
    assert!(matches!(bitset.field[3].ty, Some(BitFieldType::SignedInt)));
    assert!(matches!(bitset.field[4].ty, Some(BitFieldType::UnsignedInt)));

    let Definition::TypeDcl(type_dcl) = &hir.0[2] else { panic!("expected union"); };
    let TypeDclInner::ConstrTypeDcl(ConstrTypeDcl::UnionDef(union)) = &type_dcl.decl else {
        panic!("expected union");
    };
    assert_eq!(union.case[0].element.field_id, union.case[1].element.field_id);
    assert_ne!(union.case[0].element.field_id, union.case[2].element.field_id);
    assert!(union.case[0].element.field_id.is_some());
    assert!(union.case[2].element.field_id.is_some());

    let Definition::TypeDcl(type_dcl) = &hir.0[3] else { panic!("expected typedef"); };
    let TypeDclInner::TypedefDcl(typedef) = &type_dcl.decl else { panic!("expected typedef"); };
    assert_eq!(typedef.decl.len(), 2);

    let Definition::TypeDcl(type_dcl) = &hir.0[4] else { panic!("expected native"); };
    assert!(matches!(type_dcl.decl, TypeDclInner::NativeDcl(_)));
}

#[test]
fn hir_type_conversions_cover_simple_and_template_variants() {
    let scoped_name = typed_scoped_name(&["demo", "Thing"], false);
    let template = TypedTypeSpec::TemplateTypeSpec(TemplateTypeSpec::TemplateType(TemplateType {
        ident: Identifier("Boxed".to_string()),
        args: vec![TypedTypeSpec::SimpleTypeSpec(TypedSimpleTypeSpec::ScopedName(scoped_name))],
    }));
    let simple = TypedTypeSpec::SimpleTypeSpec(TypedSimpleTypeSpec::BaseTypeSpec(
        BaseTypeSpec::IntegerType(IntegerType::SignedInt(SignedInt::SignedLongLongInt(
            xidl_parser::typed_ast::SignedLongLongInt,
        ))),
    ));

    let template_hir: TypeSpec = template.into();
    let simple_hir: TypeSpec = simple.into();

    assert!(matches!(template_hir, TypeSpec::TemplateTypeSpec(_)));
    assert!(matches!(
        simple_hir,
        TypeSpec::SimpleTypeSpec(SimpleTypeSpec::IntegerType(xidl_parser::hir::IntegerType::I64))
    ));

    let nested = AnnotationAppl {
        name: AnnotationName::Builtin("outer".to_string()),
        params: None,
        is_extend: false,
        extra: vec![AnnotationAppl::doc("inner".to_string())],
    };
    let expanded = expand_annotations(vec![nested]);
    assert_eq!(expanded.len(), 2);
}
