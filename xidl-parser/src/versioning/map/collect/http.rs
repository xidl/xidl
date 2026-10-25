//! Collects REST and WebSocket operation surfaces from projected HIR.

use super::super::helpers::descriptor;
use super::super::model::OperationSurface;
use super::render::{join, render_method, render_param_kind, render_type};
use crate::error::ParserResult;
use crate::hir::{Specification, VisitScopedNames};
use crate::rest_hir::{self, HttpParamKind};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) fn collect_operations(
    spec: &Specification,
) -> ParserResult<BTreeMap<String, OperationSurface>> {
    let rest = rest_hir::project(spec)?;
    let mut operations = BTreeMap::new();
    for interface in rest.interfaces {
        let interface_path = join(
            &interface
                .module_path
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>()
                .join("::"),
            &interface.name,
        );
        for mut operation in interface.operations {
            if let Some(version) = interface.module_path.first() {
                operation.remove_root_namespace(version);
            }
            let params = operation
                .http
                .request
                .path
                .iter()
                .map(|param| {
                    (
                        param_key(HttpParamKind::Path, &param.wire_name),
                        param_descriptor(param, HttpParamKind::Path),
                    )
                })
                .chain(operation.http.request.query.iter().map(|param| {
                    (
                        param_key(HttpParamKind::Query, &param.wire_name),
                        param_descriptor(param, HttpParamKind::Query),
                    )
                }))
                .chain(operation.http.request.header.iter().map(|param| {
                    (
                        param_key(HttpParamKind::Header, &param.wire_name),
                        param_descriptor(param, HttpParamKind::Header),
                    )
                }))
                .chain(operation.http.request.cookie.iter().map(|param| {
                    (
                        param_key(HttpParamKind::Cookie, &param.wire_name),
                        param_descriptor(param, HttpParamKind::Cookie),
                    )
                }))
                .chain(body_params(&operation))
                .collect();
            let descriptor = descriptor([
                (
                    "method",
                    Value::String(render_method(operation.meta.method)),
                ),
                (
                    "route",
                    operation
                        .meta
                        .routes
                        .first()
                        .map(|route| Value::String(route.path.clone()))
                        .unwrap_or(Value::Null),
                ),
                (
                    "routes",
                    Value::Array(
                        operation
                            .meta
                            .routes
                            .iter()
                            .map(route_descriptor)
                            .map(Value::Object)
                            .collect(),
                    ),
                ),
                (
                    "return_type",
                    operation
                        .signature
                        .return_type
                        .as_ref()
                        .map(render_type)
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                ),
                (
                    "raises",
                    Value::Array(
                        operation
                            .meta
                            .raises
                            .iter()
                            .map(|value| {
                                Value::String(join(
                                    &value
                                        .module_path
                                        .iter()
                                        .skip(1)
                                        .cloned()
                                        .collect::<Vec<_>>()
                                        .join("::"),
                                    &value.ident,
                                ))
                            })
                            .collect(),
                    ),
                ),
                (
                    "request",
                    serde_json::to_value(&operation.http.request).unwrap_or(Value::Null),
                ),
                (
                    "response",
                    serde_json::to_value(&operation.http.response).unwrap_or(Value::Null),
                ),
                (
                    "stream",
                    serde_json::to_value(operation.meta.stream).unwrap_or(Value::Null),
                ),
                (
                    "upgrade_protocol",
                    operation
                        .meta
                        .upgrade_protocol
                        .clone()
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                ),
                (
                    "upgrade_mode",
                    serde_json::to_value(operation.meta.upgrade_mode).unwrap_or(Value::Null),
                ),
                (
                    "websocket",
                    serde_json::to_value(&operation.meta.websocket).unwrap_or(Value::Null),
                ),
            ]);
            operations.insert(
                format!("{interface_path}.{}", operation.meta.name),
                OperationSurface {
                    source: operation.meta.source,
                    parameter_sources: rest_hir::project_params::sources::ParameterSources::new(
                        operation.meta.method,
                        operation.meta.stream.kind,
                        &operation.meta.routes,
                    ),
                    descriptor,
                    params,
                },
            );
        }
    }
    Ok(operations)
}

fn body_params(operation: &rest_hir::HttpOperation) -> Vec<(String, BTreeMap<String, Value>)> {
    match &operation.http.request.body.shape {
        rest_hir::HttpRequestBodyShape::Empty => Vec::new(),
        rest_hir::HttpRequestBodyShape::SingleValue {
            source_param,
            wire_name,
            ty,
            ..
        } => vec![(
            param_key(HttpParamKind::Body, wire_name),
            descriptor([
                ("name", Value::String(source_param.clone())),
                ("location", Value::String("body".to_string())),
                ("wire_name", Value::String(wire_name.clone())),
                ("type", Value::String(render_type(ty))),
            ]),
        )],
        rest_hir::HttpRequestBodyShape::Object { fields } => fields
            .iter()
            .map(|field| {
                (
                    param_key(HttpParamKind::Body, &field.field_name),
                    descriptor([
                        ("name", Value::String(field.source_param.clone())),
                        ("location", Value::String("body".to_string())),
                        ("wire_name", Value::String(field.field_name.clone())),
                        ("type", Value::String(render_type(&field.ty))),
                        ("optional", Value::Bool(field.optional)),
                    ]),
                )
            })
            .collect(),
        rest_hir::HttpRequestBodyShape::Stream {
            source_param,
            wire_name,
            item_ty,
            ..
        } => vec![(
            param_key(HttpParamKind::Body, wire_name),
            descriptor([
                ("name", Value::String(source_param.clone())),
                ("location", Value::String("body".to_string())),
                ("wire_name", Value::String(wire_name.clone())),
                (
                    "type",
                    Value::String(format!("stream<{}>", render_type(item_ty))),
                ),
                ("optional", Value::Bool(false)),
            ]),
        )],
    }
}

fn route_descriptor(route: &rest_hir::HttpRoute) -> serde_json::Map<String, Value> {
    descriptor([
        ("path", Value::String(route.path.clone())),
        (
            "path_params",
            Value::Array(
                route
                    .path_params
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        ),
        (
            "query_params",
            Value::Array(
                route
                    .query_params
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        ),
    ])
    .into_iter()
    .collect()
}

fn param_key(kind: HttpParamKind, wire_name: &str) -> String {
    format!("{}:{}", render_param_kind(kind), wire_name)
}

fn param_descriptor(
    param: &rest_hir::HttpInputBinding,
    kind: HttpParamKind,
) -> BTreeMap<String, Value> {
    descriptor([
        ("name", Value::String(param.source_param.clone())),
        (
            "location",
            Value::String(render_param_kind(kind).to_string()),
        ),
        ("wire_name", Value::String(param.wire_name.clone())),
        ("type", Value::String(render_type(&param.ty))),
        ("optional", Value::Bool(param.optional)),
    ])
}
