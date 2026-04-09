use crate::parser::parser_text;
use crate::typed_ast::{
    AnnotationAppl, AnnotationName, AnnotationParams, Definition, TemplateTypeSpec, TypeDclInner,
    TypeDeclaratorInner, TypeSpec,
};

#[test]
fn parse_template_type_spec() {
    let typed = parser_text(
        r#"
        module m {
            typedef Vec<long> MyVec;
        };
        "#,
    )
    .expect("parse should succeed");

    let module = match &typed.0[0] {
        Definition::ModuleDcl(module) => module,
        other => panic!("expected module, got {other:?}"),
    };
    let type_dcl = match &module.definition[0] {
        Definition::TypeDcl(type_dcl) => type_dcl,
        other => panic!("expected type declaration, got {other:?}"),
    };
    let typedef = match &type_dcl.decl {
        TypeDclInner::TypedefDcl(typedef) => typedef,
        other => panic!("expected typedef, got {other:?}"),
    };
    let template = match &typedef.decl.ty {
        TypeDeclaratorInner::TemplateTypeSpec(TemplateTypeSpec::TemplateType(template)) => template,
        other => panic!("expected template_type, got {other:?}"),
    };
    assert_eq!(template.ident.0, "Vec");
    assert_eq!(template.args.len(), 1);
    assert!(matches!(
        template.args[0],
        TypeSpec::SimpleTypeSpec(crate::typed_ast::SimpleTypeSpec::BaseTypeSpec(
            crate::typed_ast::BaseTypeSpec::IntegerType(_)
        ))
    ));
}

#[test]
fn parse_doc_comments_as_doc_annotation() {
    let typed = parser_text(
        r#"
        /// module doc
        module m {
            /// struct doc
            struct S {
                /// field doc
                long x;
            };
        };
        "#,
    )
    .expect("parse should succeed");

    let module = match &typed.0[0] {
        Definition::ModuleDcl(module) => module,
        other => panic!("expected module, got {other:?}"),
    };
    assert_has_doc(&module.annotations, "\"module doc\"");

    let type_dcl = match &module.definition[0] {
        Definition::TypeDcl(type_dcl) => type_dcl,
        other => panic!("expected type declaration, got {other:?}"),
    };
    assert_has_doc(&type_dcl.annotations, "\"struct doc\"");

    let struct_def = match &type_dcl.decl {
        TypeDclInner::ConstrTypeDcl(crate::typed_ast::ConstrTypeDcl::StructDcl(
            crate::typed_ast::StructDcl::StructDef(def),
        )) => def,
        other => panic!("expected struct def, got {other:?}"),
    };
    let member = &struct_def.member[0];
    assert_has_doc(&member.annotations, "\"field doc\"");
}

fn assert_has_doc(annotations: &[AnnotationAppl], expected: &str) {
    let doc = annotations.iter().find_map(|anno| match &anno.name {
        AnnotationName::Builtin(name) if name == "doc" => match &anno.params {
            Some(AnnotationParams::Raw(raw)) => Some(raw.as_str()),
            _ => None,
        },
        _ => None,
    });
    assert_eq!(doc, Some(expected));
}
