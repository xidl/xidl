use serde_json::{Value, json};
use std::collections::HashSet;
use xidl_parser::hir;

use super::annotations::{StreamKind, doc_text, has_annotation, stream_extension_direct};
use super::methods::result_object_schema;
use super::methods_attr_support::{readonly_attr_names, validate_attr_collision};
use super::names::rpc_method_name;
use super::schema_types::schema_for_type;

pub(super) fn render_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<String>,
) -> Vec<Value> {
    let emit_watch = has_annotation(&attr.annotations, "server_stream");
    let attr_doc = doc_text(&attr.annotations);
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .map(|name| {
                render_readonly_attr(
                    spec,
                    &name,
                    interface_name,
                    module_path,
                    user_ops,
                    emit_watch,
                    attr_doc.clone(),
                )
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => render_mutable_attr(
            spec,
            interface_name,
            module_path,
            user_ops,
            emit_watch,
            attr_doc,
        ),
    }
}

fn render_readonly_attr(
    spec: &hir::ReadonlyAttrSpec,
    name: &str,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<String>,
    emit_watch: bool,
    attr_doc: Option<String>,
) -> Value {
    let getter = format!("get_attribute_{name}");
    validate_attr_collision(user_ops, name, &getter, None);

    let mut method = json!({
        "name": rpc_method_name(module_path, interface_name, &getter),
        "params": [],
        "result": {
            "name": "result",
            "schema": result_object_schema(vec![("return".to_string(), schema_for_type(&spec.ty))]),
        }
    });
    if let Some(description) = attr_doc {
        method["description"] = Value::String(description);
    }
    if emit_watch {
        method["x-xidl-stream"] = stream_extension_direct(StreamKind::Server);
    }
    method
}

fn render_mutable_attr(
    spec: &hir::AttrSpec,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<String>,
    emit_watch: bool,
    attr_doc: Option<String>,
) -> Vec<Value> {
    match &spec.declarator {
        hir::AttrDeclarator::SimpleDeclarator(list) => list
            .iter()
            .flat_map(|decl| {
                attr_methods(
                    decl.0.as_str(),
                    &spec.ty,
                    interface_name,
                    module_path,
                    user_ops,
                    emit_watch,
                    attr_doc.clone(),
                )
            })
            .collect(),
        hir::AttrDeclarator::WithRaises { declarator, .. } => attr_methods(
            declarator.0.as_str(),
            &spec.ty,
            interface_name,
            module_path,
            user_ops,
            emit_watch,
            attr_doc,
        ),
    }
}

fn attr_methods(
    name: &str,
    ty: &hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<String>,
    emit_watch: bool,
    attr_doc: Option<String>,
) -> Vec<Value> {
    let getter = format!("get_attribute_{name}");
    let setter = format!("set_attribute_{name}");
    validate_attr_collision(user_ops, name, &getter, Some(&setter));
    vec![
        getter_method(
            name,
            ty,
            interface_name,
            module_path,
            emit_watch,
            attr_doc.clone(),
        ),
        setter_method(name, ty, interface_name, module_path, attr_doc),
    ]
}

fn getter_method(
    name: &str,
    ty: &hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
    emit_watch: bool,
    attr_doc: Option<String>,
) -> Value {
    let getter = format!("get_attribute_{name}");
    let mut method = json!({
        "name": rpc_method_name(module_path, interface_name, &getter),
        "params": [],
        "result": {
            "name": "result",
            "schema": result_object_schema(vec![("return".to_string(), schema_for_type(ty))]),
        }
    });
    if let Some(description) = attr_doc {
        method["description"] = Value::String(description);
    }
    if emit_watch {
        method["x-xidl-stream"] = stream_extension_direct(StreamKind::Server);
    }
    method
}

fn setter_method(
    name: &str,
    ty: &hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
    attr_doc: Option<String>,
) -> Value {
    let setter = format!("set_attribute_{name}");
    let mut method = json!({
        "name": rpc_method_name(module_path, interface_name, &setter),
        "params": [{
            "name": name,
            "required": true,
            "schema": schema_for_type(ty),
        }],
        "result": {
            "name": "result",
            "schema": result_object_schema(Vec::new()),
        }
    });
    if let Some(description) = attr_doc {
        method["params"][0]["description"] = Value::String(description);
    }
    method
}
