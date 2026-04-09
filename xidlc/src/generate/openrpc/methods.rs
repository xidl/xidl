use serde_json::{Map, Value, json};
use xidl_parser::hir;

use super::annotations::{
    doc_text, has_optional_annotation, stream_extension, stream_kind_from_annotations,
};
use super::names::rpc_method_name;
use super::schema_types::schema_for_type;

#[derive(Copy, Clone)]
enum ParamDirection {
    In,
    Out,
    InOut,
}

fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

pub(super) fn render_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> Value {
    let mut params = Vec::new();
    let mut outputs = Vec::new();
    let stream_kind = stream_kind_from_annotations(&op.annotations);

    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        outputs.push(("return".to_string(), schema_for_type(ty)));
    }

    let param_list = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);

    for param in param_list {
        let direction = param_direction(param.attr.as_ref());
        let name = param.declarator.0.clone();
        let schema = schema_for_type(&param.ty);

        if matches!(direction, ParamDirection::In | ParamDirection::InOut) {
            let mut param_value = json!({
                "name": name,
                "required": !has_optional_annotation(&param.annotations),
                "schema": schema,
            });
            if let Some(description) = doc_text(&param.annotations) {
                param_value["description"] = Value::String(description);
            }
            params.push(param_value);
        }
        if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
            outputs.push((name, schema));
        }
    }

    let mut method = json!({
        "name": rpc_method_name(module_path, interface_name, &op.ident),
        "params": params,
        "result": {
            "name": "result",
            "schema": result_object_schema(outputs),
        },
    });
    if let Some(description) = doc_text(&op.annotations) {
        method["description"] = Value::String(description);
    }
    if let Some(kind) = stream_kind {
        method["x-xidl-stream"] = stream_extension(kind, module_path, interface_name, &op.ident);
    }
    method
}

pub(super) fn result_object_schema(fields: Vec<(String, Value)>) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for (name, schema) in fields {
        properties.insert(name.clone(), schema);
        required.push(Value::String(name));
    }

    let mut out = json!({
        "type": "object",
        "properties": properties,
    });
    if !required.is_empty() {
        out["required"] = Value::Array(required);
    }
    out
}
