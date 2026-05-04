use std::collections::HashSet;

use crate::error::ParserResult;
use crate::hir;

use super::semantics::{has_annotation, validate_attr_collision};
use super::{
    JsonRpcField, JsonRpcFieldSource, JsonRpcMethod, JsonRpcMethodKind, JsonRpcMethodSource,
    JsonRpcResponseKind, JsonRpcWatchMethod,
};

#[cfg(test)]
mod tests;

pub(super) fn project_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<&str>,
) -> ParserResult<(Vec<JsonRpcMethod>, Vec<JsonRpcWatchMethod>)> {
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => project_readonly_attr(
            spec,
            &spec.ty,
            &attr.annotations,
            interface_name,
            module_path,
            user_ops,
        ),
        hir::AttrDclInner::AttrSpec(spec) => project_readwrite_attr(
            spec,
            &spec.ty,
            &attr.annotations,
            interface_name,
            module_path,
            user_ops,
        ),
    }
}

fn project_readonly_attr(
    spec: &hir::ReadonlyAttrSpec,
    ty: &hir::TypeSpec,
    annotations: &[hir::Annotation],
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<&str>,
) -> ParserResult<(Vec<JsonRpcMethod>, Vec<JsonRpcWatchMethod>)> {
    let mut methods = Vec::new();
    let mut watch_methods = Vec::new();
    for decl in readonly_names(spec) {
        let getter = format!("get_attribute_{decl}");
        validate_attr_collision(user_ops, &decl, &getter, "")?;
        methods.push(getter_method(
            &getter,
            ty,
            annotations,
            interface_name,
            module_path,
        ));
        if has_annotation(annotations, "server_stream") {
            let stream_name = format!("set_attribute_{decl}");
            methods.push(stream_source_method(
                &stream_name,
                ty,
                annotations,
                interface_name,
                module_path,
            ));
            watch_methods.push(JsonRpcWatchMethod {
                getter_name: getter,
                item_ty: ty.clone(),
                stream_rpc_name: rpc_method_name(module_path, interface_name, &stream_name),
            });
        }
    }
    Ok((methods, watch_methods))
}

fn project_readwrite_attr(
    spec: &hir::AttrSpec,
    ty: &hir::TypeSpec,
    annotations: &[hir::Annotation],
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<&str>,
) -> ParserResult<(Vec<JsonRpcMethod>, Vec<JsonRpcWatchMethod>)> {
    let mut methods = Vec::new();
    let mut watch_methods = Vec::new();
    for decl in attr_names(spec) {
        let getter = format!("get_attribute_{decl}");
        let setter = format!("set_attribute_{decl}");
        validate_attr_collision(user_ops, &decl, &getter, &setter)?;
        methods.push(getter_method(
            &getter,
            ty,
            annotations,
            interface_name,
            module_path,
        ));
        methods.push(setter_method(
            &decl,
            &setter,
            ty,
            annotations,
            interface_name,
            module_path,
        ));
        if has_annotation(annotations, "server_stream") {
            methods.push(stream_source_method(
                &setter,
                ty,
                annotations,
                interface_name,
                module_path,
            ));
            watch_methods.push(JsonRpcWatchMethod {
                getter_name: getter,
                item_ty: ty.clone(),
                stream_rpc_name: rpc_method_name(module_path, interface_name, &setter),
            });
        }
    }
    Ok((methods, watch_methods))
}

fn getter_method(
    getter: &str,
    ty: &hir::TypeSpec,
    annotations: &[hir::Annotation],
    interface_name: &str,
    module_path: &[String],
) -> JsonRpcMethod {
    JsonRpcMethod {
        source: JsonRpcMethodSource::AttributeGet,
        kind: JsonRpcMethodKind::Unary,
        name: getter.to_string(),
        rpc_name: rpc_method_name(module_path, interface_name, getter),
        annotations: annotations.to_vec(),
        request_fields: Vec::new(),
        response_fields: vec![return_field(ty)],
        response_kind: JsonRpcResponseKind::SingleReturn,
        stream_item: None,
    }
}

fn setter_method(
    field_name: &str,
    setter: &str,
    ty: &hir::TypeSpec,
    annotations: &[hir::Annotation],
    interface_name: &str,
    module_path: &[String],
) -> JsonRpcMethod {
    JsonRpcMethod {
        source: JsonRpcMethodSource::AttributeSet,
        kind: JsonRpcMethodKind::Unary,
        name: setter.to_string(),
        rpc_name: rpc_method_name(module_path, interface_name, setter),
        annotations: annotations.to_vec(),
        request_fields: vec![JsonRpcField {
            name: field_name.to_string(),
            wire_name: field_name.to_string(),
            ty: ty.clone(),
            source: JsonRpcFieldSource::Param,
        }],
        response_fields: Vec::new(),
        response_kind: JsonRpcResponseKind::Empty,
        stream_item: None,
    }
}

fn stream_source_method(
    name: &str,
    ty: &hir::TypeSpec,
    annotations: &[hir::Annotation],
    interface_name: &str,
    module_path: &[String],
) -> JsonRpcMethod {
    JsonRpcMethod {
        source: JsonRpcMethodSource::AttributeStreamSource,
        kind: JsonRpcMethodKind::StreamSource,
        name: name.to_string(),
        rpc_name: rpc_method_name(module_path, interface_name, name),
        annotations: annotations.to_vec(),
        request_fields: Vec::new(),
        response_fields: Vec::new(),
        response_kind: JsonRpcResponseKind::Empty,
        stream_item: Some(ty.clone()),
    }
}

fn readonly_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn attr_names(spec: &hir::AttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::AttrDeclarator::SimpleDeclarator(values) => {
            values.iter().map(|value| value.0.clone()).collect()
        }
        hir::AttrDeclarator::WithRaises { declarator, .. } => vec![declarator.0.clone()],
    }
}

fn return_field(ty: &hir::TypeSpec) -> JsonRpcField {
    JsonRpcField {
        name: "return".to_string(),
        wire_name: "return".to_string(),
        ty: ty.clone(),
        source: JsonRpcFieldSource::Return,
    }
}

fn rpc_method_name(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}
