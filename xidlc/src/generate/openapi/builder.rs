use super::operation::MethodInfo;
use super::schema::error_schema_ref;
use crate::openapi::response::ResponseBuilder;
use crate::openapi::schema::{ObjectBuilder, Schema, Type};
use crate::openapi::{Content, RefOr, ResponsesBuilder};

pub(crate) fn build_operation(method: &MethodInfo) -> crate::openapi::path::Operation {
    let mut responses = ResponsesBuilder::new();
    let mut ok_response = ResponseBuilder::new().description("OK");
    if let Some(schema) = &method.response_schema {
        ok_response = ok_response.content(
            &method.response_content_type,
            Content::new(Some::<RefOr<Schema>>(schema.clone())),
        );
    }
    responses = responses.response(method.response_status, ok_response.build());

    let mut operation = crate::openapi::path::OperationBuilder::new()
        .operation_id(Some(method.operation_id.clone()))
        .deprecated(
            method
                .deprecated
                .then_some(crate::openapi::Deprecated::True),
        )
        .responses(
            responses
                .response(
                    "500",
                    ResponseBuilder::new()
                        .description("Error")
                        .content("application/json", Content::new(Some(error_schema_ref())))
                        .build(),
                )
                .build(),
        );
    if method.summary.is_some() || method.description.is_some() {
        operation = operation
            .summary(method.summary.as_deref())
            .description(method.description.as_deref());
    }
    if let Some(security) = &method.security {
        operation = operation.securities(Some(security.clone()));
    }
    for parameter in &method.parameters {
        operation = operation.parameter(parameter.clone());
    }
    if let Some(request_body) = &method.request_body {
        operation = operation.request_body(Some(request_body.clone()));
    }
    operation.build()
}

pub(crate) fn build_error_schema() -> RefOr<Schema> {
    RefOr::T(Schema::from(
        ObjectBuilder::new()
            .schema_type(Type::Object)
            .property("code", ObjectBuilder::new().schema_type(Type::Integer))
            .required("code")
            .property("msg", ObjectBuilder::new().schema_type(Type::String))
            .required("msg")
            .property("details", ObjectBuilder::new().schema_type(Type::Object)),
    ))
}
