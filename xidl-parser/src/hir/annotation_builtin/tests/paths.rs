use super::*;
use crate::typed_ast::{AnnotationAppl, AnnotationName};

#[test]
fn annotation_from_builtin_path_uses_conversion_function() {
    let appl = AnnotationAppl {
        name: AnnotationName::Builtin("id".to_string()),
        params: None,
        builtin: Some(BuiltinAnnotation::Id {
            value: TypedIntegerLiteral::BinNumber("0b101".to_string()),
        }),
        is_extend: false,
        extra: Vec::new(),
    };
    assert_eq!(
        serde_json::to_value(Annotation::from(appl)).unwrap(),
        serde_json::json!({"Id":{"value":"0b101"}})
    );
}
