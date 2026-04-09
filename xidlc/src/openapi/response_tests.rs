use crate::openapi::{Content, ResponseBuilder, Responses};
use insta::assert_json_snapshot;

#[test]
fn responses_new() {
    let responses = Responses::new();
    assert!(responses.responses.is_empty());
}

#[test]
fn response_builder() {
    let request_body = ResponseBuilder::new()
        .description("A sample response")
        .content(
            "application/json",
            Content::new(Some(crate::openapi::Ref::from_schema_name(
                "MySchemaPayload",
            ))),
        )
        .build();
    assert_json_snapshot!(request_body);
}
