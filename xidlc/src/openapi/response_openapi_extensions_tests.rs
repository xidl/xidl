use crate::openapi::{ResponseBuilder, response::ResponseExt};
use insta::assert_json_snapshot;

#[test]
fn response_ext() {
    let request_body = ResponseBuilder::new()
        .description("A sample response")
        .build()
        .json_schema_ref("MySchemaPayload");

    assert_json_snapshot!(request_body);
}

#[test]
fn response_builder_ext() {
    let request_body = ResponseBuilder::new()
        .description("A sample response")
        .json_schema_ref("MySchemaPayload")
        .build();

    assert_json_snapshot!(request_body);
}
