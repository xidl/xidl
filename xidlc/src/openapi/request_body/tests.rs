use super::{Content, RequestBody, RequestBodyBuilder, Required};
use insta::assert_json_snapshot;

#[test]
fn request_body_new() {
    let request_body = RequestBody::new();

    assert!(request_body.content.is_empty());
    assert_eq!(request_body.description, None);
    assert!(request_body.required.is_none());
}

#[test]
fn request_body_builder() {
    let request_body = RequestBodyBuilder::new()
        .description(Some("A sample requestBody"))
        .required(Some(Required::True))
        .content(
            "application/json",
            Content::new(Some(crate::openapi::Ref::from_schema_name("EmailPayload"))),
        )
        .build();
    assert_json_snapshot!(request_body);
}
