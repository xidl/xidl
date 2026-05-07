use super::{
    TransportDirection, TransportFieldContext, TransportItemContext, TransportTracker,
    TransportTypeDef, TypeRegistry,
};
use xidl_parser::hir::{
    Annotation, Declarator, IntegerType, Member, ScopedName, SimpleDeclarator, StructDcl, TypeSpec,
};

fn widget_struct() -> StructDcl {
    StructDcl {
        annotations: vec![],
        ident: "Widget".to_string(),
        parent: vec![],
        member: vec![
            Member {
                annotations: vec![Annotation::Optional { value: None }],
                ty: TypeSpec::IntegerType(IntegerType::I32),
                ident: vec![Declarator::SimpleDeclarator(SimpleDeclarator(
                    "count".to_string(),
                ))],
                default: None,
                field_id: None,
                recursive: false,
            },
            Member {
                annotations: vec![],
                ty: TypeSpec::Boolean,
                ident: vec![Declarator::SimpleDeclarator(SimpleDeclarator(
                    "enabled".to_string(),
                ))],
                default: None,
                field_id: None,
                recursive: false,
            },
        ],
    }
}

fn tracked_item_fields(item: &TransportItemContext) -> &[TransportFieldContext] {
    assert_eq!(item.kind, "struct");
    &item.fields
}

#[test]
fn render_modules_returns_structured_transport_context() {
    let mut tracker = TransportTracker::new("DemoApi");
    let registry = TypeRegistry::from([(
        "demo::Widget".to_string(),
        TransportTypeDef::Struct(widget_struct()),
    )]);

    tracker
        .map_type(
            &TypeSpec::ScopedName(ScopedName {
                name: vec!["demo".to_string(), "Widget".to_string()],
                is_root: false,
            }),
            TransportDirection::In,
            &registry,
        )
        .expect("track widget type");

    let modules = tracker
        .render_modules(&registry, &["demo".to_string()])
        .expect("render transport modules");
    let item = modules
        .inbound
        .items
        .first()
        .expect("expected one transport item");
    let fields = tracked_item_fields(item);

    assert_eq!(modules.inbound.name, "__xidl_in_DemoApi");
    assert_eq!(item.transport_ident, "demo_Widget");
    assert_eq!(item.public_path, "super::Widget");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "count");
    assert_eq!(fields[0].ty, "Option<i32>");
    assert!(fields[0].optional);
    assert_eq!(fields[0].encode_expr, "value.count");
    assert_eq!(fields[0].decode_expr, "value.count");
    assert_eq!(fields[1].name, "enabled");
    assert_eq!(fields[1].ty, "bool");
}
