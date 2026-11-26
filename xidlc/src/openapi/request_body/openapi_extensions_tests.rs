use crate::openapi::request_body::RequestBodyBuilder;
use insta::assert_json_snapshot;

use super::RequestBodyExt;

#[test]
fn request_body_ext() {
    let request_body = RequestBodyBuilder::new()
        .build()
        // build a RequestBody first to test the method
        .json_schema_ref("EmailPayload");
    assert_json_snapshot!(request_body);
}

#[test]
fn request_body_builder_ext() {
    let request_body = RequestBodyBuilder::new()
        .json_schema_ref("EmailPayload")
        .build();
    assert_json_snapshot!(request_body);
}
