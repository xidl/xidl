use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_jsonrpc::{JsonRpcRenderOutput, JsonRpcRenderer};
use convert_case::{Case, Casing};
use itertools::Itertools;
use serde::Serialize;
use std::collections::HashSet;
use xidl_parser::hir;

#[derive(Serialize)]
struct ParamField {
    name: String,
    ty: String,
}

#[derive(Serialize)]
struct OutputField {
    name: String,
    json_name: String,
    ty: String,
}

#[derive(Serialize)]
struct MethodContext {
    kind: String,
    stream_mode: String,
    name: String,
    params: Vec<String>,
    params_fields: Vec<ParamField>,
    params_struct: String,
    ret: String,
    rpc_name: String,
    args: Vec<String>,
    response_kind: String,
    response_struct: String,
    response_fields: Vec<OutputField>,
    response_single_field: String,
    stream_item_ty: String,
}

#[derive(Serialize)]
struct WatchMethodContext {
    getter_name: String,
    item_ty: String,
    stream_rpc_name: String,
}

fn stream_mode_name(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Server => "server",
        StreamKind::Client => "client",
        StreamKind::Bidi => "bidi",
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum StreamKind {
    Server,
    Client,
    Bidi,
}

pub fn render_interface_with_path(
    interface: &hir::InterfaceDcl,
    renderer: &JsonRpcRenderer,
    module_path: &[String],
) -> IdlcResult<JsonRpcRenderOutput> {
    match &interface.decl {
        hir::InterfaceDclInner::InterfaceForwardDcl(_) => Ok(JsonRpcRenderOutput::default()),
        hir::InterfaceDclInner::InterfaceDef(def) => {
            render_interface_def(def, renderer, module_path)
        }
    }
}

fn render_interface_def(
    def: &hir::InterfaceDef,
    renderer: &JsonRpcRenderer,
    module_path: &[String],
) -> IdlcResult<JsonRpcRenderOutput> {
    let mut out = JsonRpcRenderOutput::default();
    let mut methods = Vec::new();
    let mut watch_methods = Vec::new();
    let mut user_ops = HashSet::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            if let hir::Export::OpDcl(op) = export {
                user_ops.insert(op.ident.as_str());
            }
        }
    }

    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    methods.extend(render_op(op, &def.header.ident, module_path)?);
                }
                hir::Export::AttrDcl(attr) => {
                    let rendered = render_attr(attr, &def.header.ident, module_path, &user_ops)?;
                    methods.extend(rendered.methods);
                    watch_methods.extend(rendered.watch_methods);
                }
                _ => {}
            }
        }
    }

    let ctx = serde_json::json!({
        "ident": rust_ident(&def.header.ident),
        "methods": methods,
        "watch_methods": watch_methods,
    });
    let rendered = renderer.render_template("interface.rs.j2", &ctx)?;
    out.source.push(rendered);
    Ok(out)
}

fn render_op(
    op: &hir::OpDcl,
    interface_name: &str,
    module_path: &[String],
) -> IdlcResult<Vec<MethodContext>> {
    let stream_kind = stream_kind_from_annotations(&op.annotations)?;
    if let Some(kind) = stream_kind {
        return Ok(vec![render_stream_op(
            op,
            interface_name,
            module_path,
            kind,
        )]);
    }
    Ok(vec![render_unary_op(op, interface_name, module_path)])
}

fn render_unary_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> MethodContext {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut param_list = Vec::new();
    let mut param_fields = Vec::new();
    let mut args = Vec::new();
    let mut response_fields = Vec::new();

    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        response_fields.push(OutputField {
            name: rust_ident("return"),
            json_name: "return".to_string(),
            ty: jsonrpc_type(ty),
        });
    }

    for param in params {
        let mode = param_mode(param.attr.as_ref());
        let ty = render_param_type(&param.ty, param.attr.as_ref());
        let raw_name = param.declarator.0.as_str();
        let name = rust_ident(raw_name);
        if matches!(mode, ParamMode::In | ParamMode::InOut) {
            param_list.push(format!("{name}: {ty}"));
            param_fields.push(ParamField {
                name: name.clone(),
                ty: ty.clone(),
            });
            args.push(name);
        }
        if matches!(mode, ParamMode::Out | ParamMode::InOut) {
            response_fields.push(OutputField {
                name: rust_ident(raw_name),
                json_name: raw_name.to_string(),
                ty,
            });
        }
    }

    let method_name = rust_ident(&op.ident);
    let response_kind = if response_fields.is_empty() {
        "empty"
    } else if response_fields.len() == 1 && response_fields[0].json_name == "return" {
        "single_return"
    } else if response_fields.len() == 1 {
        "single_output"
    } else {
        "multi"
    };
    let ret = match response_kind {
        "empty" => "()".to_string(),
        "single_return" | "single_output" => response_fields[0].ty.clone(),
        _ => response_struct_name(interface_name, &method_name),
    };
    MethodContext {
        kind: "rpc".to_string(),
        stream_mode: String::new(),
        name: method_name.clone(),
        params: param_list,
        params_fields: param_fields,
        params_struct: params_struct_name(interface_name, &method_name),
        ret,
        rpc_name: rpc_method_name(module_path, interface_name, &op.ident),
        args,
        response_kind: response_kind.to_string(),
        response_struct: response_struct_name(interface_name, &method_name),
        response_single_field: response_fields
            .first()
            .map(|value| value.name.clone())
            .unwrap_or_default(),
        response_fields,
        stream_item_ty: String::new(),
    }
}

fn render_stream_op(
    op: &hir::OpDcl,
    interface_name: &str,
    module_path: &[String],
    kind: StreamKind,
) -> MethodContext {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut param_list = Vec::new();
    let mut param_fields = Vec::new();
    let mut args = Vec::new();
    let mut response_fields = Vec::new();

    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        response_fields.push(OutputField {
            name: rust_ident("return"),
            json_name: "return".to_string(),
            ty: jsonrpc_type(ty),
        });
    }

    for param in params {
        let mode = param_mode(param.attr.as_ref());
        if matches!(mode, ParamMode::In | ParamMode::InOut) {
            let name = rust_ident(&param.declarator.0);
            let ty = render_param_type(&param.ty, param.attr.as_ref());
            param_list.push(format!("{name}: {ty}"));
            param_fields.push(ParamField {
                name: name.clone(),
                ty: ty.clone(),
            });
            args.push(name);
        }
        if matches!(mode, ParamMode::Out | ParamMode::InOut) {
            response_fields.push(OutputField {
                name: rust_ident(&param.declarator.0),
                json_name: param.declarator.0.clone(),
                ty: render_param_type(&param.ty, param.attr.as_ref()),
            });
        }
    }

    let response_kind = if response_fields.is_empty() {
        "empty"
    } else if response_fields.len() == 1 && response_fields[0].json_name == "return" {
        "single_return"
    } else if response_fields.len() == 1 {
        "single_output"
    } else {
        "multi"
    };
    let unary_ret = match response_kind {
        "empty" => "()".to_string(),
        "single_return" | "single_output" => response_fields[0].ty.clone(),
        _ => response_struct_name(interface_name, &op.ident),
    };

    let method_name = rust_ident(&op.ident);
    let (params, ret) = match kind {
        StreamKind::Server => (
            param_list,
            "xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>".to_string(),
        ),
        StreamKind::Client => (
            vec!["stream: xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>".to_string()],
            unary_ret,
        ),
        StreamKind::Bidi => (
            vec![
                "stream: xidl_jsonrpc::stream::ReaderWriter<serde_json::Value, serde_json::Value>"
                    .to_string(),
            ],
            "()".to_string(),
        ),
    };
    let response_single_field = response_fields
        .first()
        .map(|value| value.name.clone())
        .unwrap_or_default();

    MethodContext {
        kind: "stream_op".to_string(),
        stream_mode: stream_mode_name(kind).to_string(),
        name: method_name.clone(),
        params,
        params_fields: if matches!(kind, StreamKind::Server) {
            param_fields
        } else {
            Vec::new()
        },
        params_struct: if matches!(kind, StreamKind::Server) {
            params_struct_name(interface_name, &method_name)
        } else {
            String::new()
        },
        ret,
        rpc_name: rpc_method_name(module_path, interface_name, &op.ident),
        args: if matches!(kind, StreamKind::Server) {
            args
        } else {
            Vec::new()
        },
        response_kind: response_kind.to_string(),
        response_struct: response_struct_name(interface_name, &method_name),
        response_fields,
        response_single_field,
        stream_item_ty: String::new(),
    }
}

struct RenderedAttr {
    methods: Vec<MethodContext>,
    watch_methods: Vec<WatchMethodContext>,
}

fn render_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<&str>,
) -> IdlcResult<RenderedAttr> {
    let emit_watch = has_annotation(&attr.annotations, "server_stream");
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => {
            let mut out = Vec::new();
            let mut watch_methods = Vec::new();
            for names in readonly_attr_names(spec) {
                validate_attr_collision(user_ops, &names.raw_attr, &names.raw_getter, "")?;
                let ret = attr_return_type(&spec.ty);
                out.push(MethodContext {
                    kind: "rpc".to_string(),
                    stream_mode: String::new(),
                    name: rust_ident(&names.raw_getter),
                    params: Vec::new(),
                    params_fields: Vec::new(),
                    params_struct: params_struct_name(interface_name, &names.raw_getter),
                    ret,
                    rpc_name: rpc_method_name(module_path, interface_name, &names.raw_getter),
                    args: Vec::new(),
                    response_kind: "single_return".to_string(),
                    response_struct: response_struct_name(interface_name, &names.raw_getter),
                    response_fields: vec![OutputField {
                        name: rust_ident("return"),
                        json_name: "return".to_string(),
                        ty: attr_return_type(&spec.ty),
                    }],
                    response_single_field: rust_ident("return"),
                    stream_item_ty: String::new(),
                });
                if emit_watch {
                    let raw_stream_setter = format!("set_attribute_{}", names.raw_attr);
                    let stream_setter = rust_ident(&raw_stream_setter);
                    out.push(MethodContext {
                        kind: "stream_source".to_string(),
                        stream_mode: String::new(),
                        name: stream_setter,
                        params: Vec::new(),
                        params_fields: Vec::new(),
                        params_struct: String::new(),
                        ret: String::new(),
                        rpc_name: rpc_method_name(module_path, interface_name, &raw_stream_setter),
                        args: Vec::new(),
                        response_kind: "empty".to_string(),
                        response_struct: String::new(),
                        response_fields: Vec::new(),
                        response_single_field: String::new(),
                        stream_item_ty: attr_return_type(&spec.ty),
                    });
                    watch_methods.push(WatchMethodContext {
                        getter_name: rust_ident(&names.raw_getter),
                        item_ty: attr_return_type(&spec.ty),
                        stream_rpc_name: rpc_method_name(
                            module_path,
                            interface_name,
                            &raw_stream_setter,
                        ),
                    });
                }
            }
            Ok(RenderedAttr {
                methods: out,
                watch_methods,
            })
        }
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            let mut watch_methods = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let raw_name = decl.0.clone();
                        let raw_getter = format!("get_attribute_{raw_name}");
                        let raw_setter = format!("set_attribute_{raw_name}");
                        validate_attr_collision(user_ops, &raw_name, &raw_getter, &raw_setter)?;
                        let getter = rust_ident(&raw_getter);
                        let setter = rust_ident(&raw_setter);
                        let ret = attr_return_type(&spec.ty);
                        let param = render_param_type(&spec.ty, None);
                        let value_ident = rust_ident(&raw_name);
                        out.push(MethodContext {
                            kind: "rpc".to_string(),
                            stream_mode: String::new(),
                            name: getter.clone(),
                            params: Vec::new(),
                            params_fields: Vec::new(),
                            params_struct: params_struct_name(interface_name, &getter),
                            ret,
                            rpc_name: rpc_method_name(module_path, interface_name, &raw_getter),
                            args: Vec::new(),
                            response_kind: "single_return".to_string(),
                            response_struct: response_struct_name(interface_name, &getter),
                            response_fields: vec![OutputField {
                                name: rust_ident("return"),
                                json_name: "return".to_string(),
                                ty: attr_return_type(&spec.ty),
                            }],
                            response_single_field: rust_ident("return"),
                            stream_item_ty: String::new(),
                        });
                        out.push(MethodContext {
                            kind: "rpc".to_string(),
                            stream_mode: String::new(),
                            name: setter.clone(),
                            params: vec![format!("{value_ident}: {param}")],
                            params_fields: vec![ParamField {
                                name: value_ident.clone(),
                                ty: param,
                            }],
                            params_struct: params_struct_name(interface_name, &setter),
                            ret: "()".to_string(),
                            rpc_name: rpc_method_name(module_path, interface_name, &raw_setter),
                            args: vec![value_ident],
                            response_kind: "empty".to_string(),
                            response_struct: response_struct_name(interface_name, &setter),
                            response_fields: Vec::new(),
                            response_single_field: String::new(),
                            stream_item_ty: String::new(),
                        });
                        if emit_watch {
                            let raw_stream_setter = format!("set_attribute_{raw_name}");
                            let stream_setter = rust_ident(&raw_stream_setter);
                            out.push(MethodContext {
                                kind: "stream_source".to_string(),
                                stream_mode: String::new(),
                                name: stream_setter,
                                params: Vec::new(),
                                params_fields: Vec::new(),
                                params_struct: String::new(),
                                ret: String::new(),
                                rpc_name: rpc_method_name(
                                    module_path,
                                    interface_name,
                                    &raw_stream_setter,
                                ),
                                args: Vec::new(),
                                response_kind: "empty".to_string(),
                                response_struct: String::new(),
                                response_fields: Vec::new(),
                                response_single_field: String::new(),
                                stream_item_ty: attr_return_type(&spec.ty),
                            });
                            watch_methods.push(WatchMethodContext {
                                getter_name: getter,
                                item_ty: attr_return_type(&spec.ty),
                                stream_rpc_name: rpc_method_name(
                                    module_path,
                                    interface_name,
                                    &raw_stream_setter,
                                ),
                            });
                        }
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let raw_name = declarator.0.clone();
                    let raw_getter = format!("get_attribute_{raw_name}");
                    let raw_setter = format!("set_attribute_{raw_name}");
                    validate_attr_collision(user_ops, &raw_name, &raw_getter, &raw_setter)?;
                    let getter = rust_ident(&raw_getter);
                    let setter = rust_ident(&raw_setter);
                    let ret = attr_return_type(&spec.ty);
                    let param = render_param_type(&spec.ty, None);
                    let value_ident = rust_ident(&raw_name);
                    out.push(MethodContext {
                        kind: "rpc".to_string(),
                        stream_mode: String::new(),
                        name: getter.clone(),
                        params: Vec::new(),
                        params_fields: Vec::new(),
                        params_struct: params_struct_name(interface_name, &getter),
                        ret,
                        rpc_name: rpc_method_name(module_path, interface_name, &raw_getter),
                        args: Vec::new(),
                        response_kind: "single_return".to_string(),
                        response_struct: response_struct_name(interface_name, &getter),
                        response_fields: vec![OutputField {
                            name: rust_ident("return"),
                            json_name: "return".to_string(),
                            ty: attr_return_type(&spec.ty),
                        }],
                        response_single_field: rust_ident("return"),
                        stream_item_ty: String::new(),
                    });
                    out.push(MethodContext {
                        kind: "rpc".to_string(),
                        stream_mode: String::new(),
                        name: setter.clone(),
                        params: vec![format!("{value_ident}: {param}")],
                        params_fields: vec![ParamField {
                            name: value_ident.clone(),
                            ty: param,
                        }],
                        params_struct: params_struct_name(interface_name, &setter),
                        ret: "()".to_string(),
                        rpc_name: rpc_method_name(module_path, interface_name, &raw_setter),
                        args: vec![value_ident],
                        response_kind: "empty".to_string(),
                        response_struct: response_struct_name(interface_name, &setter),
                        response_fields: Vec::new(),
                        response_single_field: String::new(),
                        stream_item_ty: String::new(),
                    });
                    if emit_watch {
                        let raw_stream_setter = format!("set_attribute_{raw_name}");
                        let stream_setter = rust_ident(&raw_stream_setter);
                        out.push(MethodContext {
                            kind: "stream_source".to_string(),
                            stream_mode: String::new(),
                            name: stream_setter,
                            params: Vec::new(),
                            params_fields: Vec::new(),
                            params_struct: String::new(),
                            ret: String::new(),
                            rpc_name: rpc_method_name(
                                module_path,
                                interface_name,
                                &raw_stream_setter,
                            ),
                            args: Vec::new(),
                            response_kind: "empty".to_string(),
                            response_struct: String::new(),
                            response_fields: Vec::new(),
                            response_single_field: String::new(),
                            stream_item_ty: attr_return_type(&spec.ty),
                        });
                        watch_methods.push(WatchMethodContext {
                            getter_name: getter,
                            item_ty: attr_return_type(&spec.ty),
                            stream_rpc_name: rpc_method_name(
                                module_path,
                                interface_name,
                                &raw_stream_setter,
                            ),
                        });
                    }
                }
            }
            Ok(RenderedAttr {
                methods: out,
                watch_methods,
            })
        }
    }
}

struct AttrNames {
    raw_attr: String,
    raw_getter: String,
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<AttrNames> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => {
            vec![AttrNames {
                raw_attr: decl.0.clone(),
                raw_getter: format!("get_attribute_{}", decl.0),
            }]
        }
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn attr_return_type(ty: &hir::TypeSpec) -> String {
    jsonrpc_type(ty)
}

fn render_param_type(ty: &hir::TypeSpec, attr: Option<&hir::ParamAttribute>) -> String {
    let _ = attr;
    jsonrpc_type(ty)
}

fn response_struct_name(interface_name: &str, method_name: &str) -> String {
    let method_name = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}Result",
        rust_ident(interface_name),
        method_name.to_case(Case::Camel)
    )
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ParamMode {
    In,
    Out,
    InOut,
}

fn param_mode(attr: Option<&hir::ParamAttribute>) -> ParamMode {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamMode::Out,
        Some("inout") => ParamMode::InOut,
        _ => ParamMode::In,
    }
}

fn stream_kind_from_annotations(annotations: &[hir::Annotation]) -> IdlcResult<Option<StreamKind>> {
    let mut out = None;
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("server_stream") {
            Some(StreamKind::Server)
        } else if name.eq_ignore_ascii_case("client_stream") {
            Some(StreamKind::Client)
        } else if name.eq_ignore_ascii_case("bidi_stream") {
            Some(StreamKind::Bidi)
        } else {
            None
        };
        let Some(current) = current else {
            continue;
        };
        match out {
            None => out = Some(current),
            Some(prev) if prev == current => {}
            Some(_) => {
                return Err(IdlcError::rpc(
                    "@server_stream/@client_stream/@bidi_stream are mutually exclusive",
                ));
            }
        }
    }
    Ok(out)
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

fn validate_attr_collision(
    user_ops: &HashSet<&str>,
    attr_name: &str,
    getter: &str,
    setter: &str,
) -> IdlcResult<()> {
    let getter_conflict = user_ops.contains(getter);
    let setter_conflict = !setter.is_empty() && user_ops.contains(setter);
    if getter_conflict || setter_conflict {
        let conflict = if setter.is_empty() {
            format!("`{getter}`")
        } else {
            format!("`{getter}` or `{setter}`")
        };
        return Err(IdlcError::fmt(format!(
            "attribute `{attr_name}` conflicts with user-defined operation `{conflict}`"
        )));
    }
    Ok(())
}

fn params_struct_name(interface_name: &str, method_name: &str) -> String {
    let method_name = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}Params",
        rust_ident(interface_name),
        method_name.to_case(Case::Camel)
    )
}

fn jsonrpc_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => rust_integer_type(value),
            hir::SimpleTypeSpec::FloatingPtType => "f64".to_string(),
            hir::SimpleTypeSpec::CharType => "char".to_string(),
            hir::SimpleTypeSpec::WideCharType => "char".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::AnyType => "serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ObjectType => "serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ValueBaseType => "serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => render_scoped_name(value),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                format!("Vec<{}>", jsonrpc_type(&seq.ty))
            }
            hir::TemplateTypeSpec::StringType(_) => "String".to_string(),
            hir::TemplateTypeSpec::WideStringType(_) => "String".to_string(),
            hir::TemplateTypeSpec::FixedPtType(_) => "f64".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                format!(
                    "BTreeMap<{}, {}>",
                    jsonrpc_type(&map.key),
                    jsonrpc_type(&map.value)
                )
            }
            hir::TemplateTypeSpec::TemplateType(value) => format!(
                "{}<{}>",
                rust_ident(&value.ident),
                value
                    .args
                    .iter()
                    .map(jsonrpc_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
    }
}

fn render_scoped_name(value: &hir::ScopedName) -> String {
    let path = value.name.iter().map(|part| rust_ident(part)).join("::");
    if value.is_root {
        format!("::{path}")
    } else {
        path
    }
}

fn rust_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "i8".to_string(),
        hir::IntegerType::UChar => "u8".to_string(),
        hir::IntegerType::U8 => "u8".to_string(),
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I8 => "i8".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}

fn rpc_method_name(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}
