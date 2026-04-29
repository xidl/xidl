use crate::openapi::RefOr;
use crate::openapi::request_body::RequestBody;
use crate::openapi::schema::Schema;
use serde_json::Value;

pub(crate) struct OpenApiStreamPatch {
    pub(crate) path: String,
    pub(crate) method: &'static str,
    pub(crate) response_status: Option<&'static str>,
    pub(crate) content_type: String,
    pub(crate) item_schema: Value,
}

pub(crate) fn request_stream_content_type(request_body: &Option<RequestBody>) -> String {
    request_body
        .as_ref()
        .and_then(|body| body.content.keys().next().cloned())
        .unwrap_or_else(|| panic!("stream request body is missing content"))
}

pub(crate) fn stream_patch_item_schema(schema: &RefOr<Schema>, direction: &str) -> Value {
    match serde_json::to_value(schema) {
        Ok(value) => value,
        Err(err) => panic!("failed to serialize {direction} stream schema: {err}"),
    }
}

pub(crate) fn patch_openapi_stream_content(doc: &mut Value, patch: &OpenApiStreamPatch) {
    let Some(paths) = doc.get_mut("paths").and_then(Value::as_object_mut) else {
        return;
    };
    let Some(path_item) = paths.get_mut(&patch.path).and_then(Value::as_object_mut) else {
        return;
    };
    let Some(operation) = path_item
        .get_mut(patch.method)
        .and_then(Value::as_object_mut)
    else {
        return;
    };

    let target = if let Some(status) = patch.response_status {
        operation
            .get_mut("responses")
            .and_then(Value::as_object_mut)
            .and_then(|responses| responses.get_mut(status))
            .and_then(Value::as_object_mut)
    } else {
        operation
            .get_mut("requestBody")
            .and_then(Value::as_object_mut)
    };
    let Some(target) = target else {
        return;
    };
    let Some(content) = target.get_mut("content").and_then(Value::as_object_mut) else {
        return;
    };
    let Some(media_type) = content
        .get_mut(&patch.content_type)
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    media_type.insert("itemSchema".to_string(), patch.item_schema.clone());
    media_type.remove("schema");
}
