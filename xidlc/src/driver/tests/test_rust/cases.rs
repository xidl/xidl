use serde_json::json;

pub(super) fn get_test_cases() -> Vec<(&'static str, &'static str, serde_json::Value)> {
    vec![
        (
            "simple_union",
            r#"
            enum Tag {
                A,
                B,
            };
            union SimpleUnion switch (Tag) {
                case A: int a;
                case B: int b;
            };
        "#,
            json!({
                "enable_serialize": false,
                "enable_deserialize": false,
                "enable_render_header": false
            }),
        ),
        (
            "option",
            r#"
            struct A {
                Option<string> a;
            };
            "#,
            json!({
                "enable_serialize": false,
                "enable_deserialize": false,
                "enable_render_header": false
            }),
        ), //
    ]
}
