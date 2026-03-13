use super::*;
use xidl_parser::hir;

fn doc_annotation(text: &str) -> hir::Annotation {
    hir::Annotation::Builtin {
        name: "doc".to_string(),
        params: Some(hir::AnnotationParams::Raw(format!("\"{}\"", text))),
    }
}

#[test]
fn schema_for_struct_applies_doc_to_fields() {
    let member = hir::Member {
        annotations: vec![doc_annotation("field doc")],
        ty: hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::IntegerType(hir::IntegerType::I32)),
        ident: vec![hir::Declarator::SimpleDeclarator(hir::SimpleDeclarator(
            "value".to_string(),
        ))],
        default: None,
        field_id: None,
    };
    let schema = schema_for_struct(&[member]);
    let Value::Object(map) = schema else {
        panic!("expected object schema");
    };
    let Value::Object(props) = map.get("properties").expect("properties") else {
        panic!("expected properties");
    };
    let Value::Object(value_schema) = props.get("value").expect("value") else {
        panic!("expected value schema");
    };
    assert_eq!(
        value_schema.get("description").and_then(Value::as_str),
        Some("field doc")
    );
}
