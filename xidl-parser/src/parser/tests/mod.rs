use crate::annotations::AnnotationDefinition;
use crate::parser::{normalize_source_for_tree_sitter, parser_text};
use crate::typed_ast::{
    AnnotationAppl, AnnotationApplParam, AnnotationName, AnnotationParams, BuiltinAnnotation,
    DataRepresentationKind, Definition, TemplateTypeSpec, TopicPlatform, TypeDclInner,
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

#[test]
fn parse_builtin_annotations_with_structured_args() {
    let typed = parser_text(
        r#"
        @range(min=1, max=8)
        @topic("orders", DDS)
        @data_representation(allowed_kinds = XML)
        struct S {};
        "#,
    )
    .expect("parse should succeed");

    let Definition::TypeDcl(type_dcl) = &typed.0[0] else {
        panic!("expected type declaration");
    };
    let [range, topic, data_representation] = &type_dcl.annotations[..] else {
        panic!("expected three annotations");
    };

    let range = range
        .builtin
        .as_ref()
        .expect("range should keep builtin parse");
    assert!(matches!(
        range,
        BuiltinAnnotation::Range { min, max }
            if matches!(&min.0, crate::typed_ast::ConstExpr(_))
            && matches!(&max.0, crate::typed_ast::ConstExpr(_))
    ));

    let topic = topic
        .builtin
        .as_ref()
        .expect("topic should keep builtin parse");
    assert!(matches!(
        topic,
        BuiltinAnnotation::Topic {
            name: Some(_),
            platform: Some(TopicPlatform::Dds)
        }
    ));

    let data_representation = data_representation
        .builtin
        .as_ref()
        .expect("data_representation should keep builtin parse");
    assert!(matches!(
        data_representation,
        BuiltinAnnotation::DataRepresentation { kinds }
            if kinds == &[DataRepresentationKind::Xml]
    ));
}

#[test]
fn parse_builtin_annotation_with_multiple_positional_literals_as_param_nodes() {
    let typed = parser_text(
        r#"
        @cors("https://console.example.com", "https://admin.example.com")
        interface UserApi {};
        "#,
    )
    .expect("parse should succeed");

    let Definition::InterfaceDcl(interface) = &typed.0[0] else {
        panic!("expected interface declaration");
    };
    let [cors] = &interface.annotations[..] else {
        panic!("expected one annotation");
    };

    let params = match &cors.params {
        Some(AnnotationParams::Params(params)) => params,
        other => panic!("expected param nodes, got {other:?}"),
    };
    assert!(matches!(
        params.as_slice(),
        [
            AnnotationApplParam::Positional(_),
            AnnotationApplParam::Positional(_)
        ]
    ));
}

#[test]
fn parse_hyphenated_builtin_annotations_with_bracketed_raw_arguments() {
    let normalized = normalize_source_for_tree_sitter("@client-stream interface Clock {};");
    assert_eq!(normalized, "@client_stream interface Clock {};");
    assert_eq!(
        AnnotationDefinition::lookup("client-stream").map(|definition| definition.name),
        Some("client_stream")
    );
    assert!(AnnotationDefinition::lookup("unknown-annotation").is_none());

    let literal_and_comment = r#"const string S = "@client-stream"; // @client-stream"#;
    assert_eq!(
        normalize_source_for_tree_sitter(literal_and_comment),
        literal_and_comment
    );

    let typed = parser_text(
        r#"
        @client-stream
        interface Clock {
            @oauth2(scopes = ["read"])
            void now();
        };
        "#,
    )
    .expect("normalized annotations should parse");

    let Definition::InterfaceDcl(interface) = &typed.0[0] else {
        panic!("expected interface declaration");
    };
    assert_eq!(interface.annotations.len(), 1);
    assert!(matches!(
        (&interface.annotations[0].name, &interface.annotations[0].params),
        (AnnotationName::Builtin(name), None) if name == "client-stream"
    ));

    let crate::typed_ast::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        panic!("expected interface definition");
    };
    let exports = &def.interface_body.as_ref().expect("interface body").0;
    let [crate::typed_ast::Export::OpDcl(op)] = exports.as_slice() else {
        panic!("expected single operation export");
    };
    assert_eq!(op.ident.0, "now");
    assert_eq!(op.annotations.len(), 1);
    assert!(matches!(
        (&op.annotations[0].name, &op.annotations[0].params),
        (AnnotationName::Builtin(name), Some(AnnotationParams::Raw(raw)))
            if name == "oauth2" && raw == r#"scopes = ["read"]"#
    ));
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
