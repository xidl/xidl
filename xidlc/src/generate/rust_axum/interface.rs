use crate::error::IdlcResult;
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::{RustAxumRenderOutput, RustAxumRenderer};
use convert_case::{Case, Casing};
use itertools::Itertools;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use xidl_parser::hir;

#[derive(Clone, Copy)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Clone, Copy)]
enum ParamSource {
    Path,
    Query,
    Body,
}

#[derive(Serialize)]
struct MethodContext {
    name: String,
    raw_name: String,
    params: Vec<String>,
    param_names: Vec<String>,
    ret: String,
    http_method: String,
    http_method_fn: String,
    reqwest_method: String,
    path: String,
    struct_prefix: String,
    path_params: Vec<ParamContext>,
    query_params: Vec<ParamContext>,
    body_params: Vec<ParamContext>,
    request_ty: String,
    request_struct: Option<String>,
    request_params: Vec<ParamContext>,
}

#[derive(Serialize, Clone)]
struct ParamContext {
    name: String,
    raw_name: String,
    ty: String,
    source: String,
    serde_rename: Option<String>,
}

pub fn render_interface_with_path(
    interface: &hir::InterfaceDcl,
    renderer: &RustAxumRenderer,
    module_path: &[String],
) -> IdlcResult<RustAxumRenderOutput> {
    match &interface.decl {
        hir::InterfaceDclInner::InterfaceForwardDcl(_) => Ok(RustAxumRenderOutput::default()),
        hir::InterfaceDclInner::InterfaceDef(def) => {
            render_interface_def(def, renderer, module_path)
        }
    }
}

fn render_interface_def(
    def: &hir::InterfaceDef,
    renderer: &RustAxumRenderer,
    module_path: &[String],
) -> IdlcResult<RustAxumRenderOutput> {
    let mut out = RustAxumRenderOutput::default();
    let mut methods = Vec::new();

    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    methods.push(render_op(op, &def.header.ident, module_path));
                }
                hir::Export::AttrDcl(attr) => {
                    methods.extend(render_attr(attr, &def.header.ident, module_path));
                }
                _ => {}
            }
        }
    }

    let ctx = serde_json::json!({
        "ident": rust_ident(&def.header.ident),
        "methods": methods,
    });
    let rendered = renderer.render_template("interface.rs.j2", &ctx)?;
    out.source.push(rendered);
    Ok(out)
}

fn render_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> MethodContext {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => axum_type(ty),
    };
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut param_list = Vec::new();
    let mut param_names = Vec::new();
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut body_params = Vec::new();
    let mut request_params = Vec::new();

    let (method, path) = route_from_annotations(
        &op.annotations,
        HttpMethod::Post,
        default_path(module_path, interface_name, &op.ident),
    );
    let path_param_names = parse_path_params(&path);
    let default_source = default_param_source(method);

    for param in params {
        let ty = render_param_type(&param.ty, param.attr.as_ref());
        let name = rust_ident(&param.declarator.0);
        param_list.push(format!("{name}: {ty}"));
        param_names.push(name.clone());
        let source = if path_param_names.contains(&param.declarator.0) {
            ParamSource::Path
        } else {
            default_source
        };
        let ctx = ParamContext {
            name: name.clone(),
            raw_name: param.declarator.0.clone(),
            ty,
            source: param_source_code(source),
            serde_rename: serde_rename(&param.declarator.0, &name),
        };
        request_params.push(ctx.clone());
        match source {
            ParamSource::Path => path_params.push(ctx),
            ParamSource::Query => query_params.push(ctx),
            ParamSource::Body => body_params.push(ctx),
        }
    }

    let method_name = rust_ident(&op.ident);
    let request_struct = if request_params.is_empty() {
        None
    } else {
        Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &op.ident)
        ))
    };
    let request_ty = request_struct.clone().unwrap_or_else(|| "()".to_string());
    MethodContext {
        name: method_name,
        raw_name: op.ident.clone(),
        params: param_list,
        param_names,
        ret,
        http_method: http_method_code(method),
        http_method_fn: http_method_fn(method),
        reqwest_method: reqwest_method_code(method),
        path,
        struct_prefix: method_struct_prefix(interface_name, &op.ident),
        path_params,
        query_params,
        body_params,
        request_ty,
        request_struct,
        request_params,
    }
}

fn render_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
) -> Vec<MethodContext> {
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .map(|names| {
                let ret = attr_return_type(&spec.ty);
                let raw = names.raw.clone();
                let path = default_path(module_path, interface_name, &raw);
                let request_struct = None;
                MethodContext {
                    name: names.rust.clone(),
                    raw_name: raw.clone(),
                    params: Vec::new(),
                    param_names: Vec::new(),
                    ret,
                    http_method: http_method_code(HttpMethod::Get),
                    http_method_fn: http_method_fn(HttpMethod::Get),
                    reqwest_method: reqwest_method_code(HttpMethod::Get),
                    path,
                    struct_prefix: method_struct_prefix(interface_name, &raw),
                    path_params: Vec::new(),
                    query_params: Vec::new(),
                    body_params: Vec::new(),
                    request_ty: "()".to_string(),
                    request_struct,
                    request_params: Vec::new(),
                }
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let name = rust_ident(&decl.0);
                        let raw_name = decl.0.clone();
                        let ret = attr_return_type(&spec.ty);
                        let path = default_path(module_path, interface_name, &raw_name);
                        let request_struct = None;
                        out.push(MethodContext {
                            name: name.clone(),
                            raw_name: raw_name.clone(),
                            params: Vec::new(),
                            param_names: Vec::new(),
                            ret,
                            http_method: http_method_code(HttpMethod::Get),
                            http_method_fn: http_method_fn(HttpMethod::Get),
                            reqwest_method: reqwest_method_code(HttpMethod::Get),
                            path,
                            struct_prefix: method_struct_prefix(interface_name, &raw_name),
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            body_params: Vec::new(),
                            request_ty: "()".to_string(),
                            request_struct,
                            request_params: Vec::new(),
                        });
                        let raw_setter = format!("set_{raw_name}");
                        let setter = rust_ident(&raw_setter);
                        let param = render_param_type(&spec.ty, None);
                        let setter_path = default_path(module_path, interface_name, &raw_setter);
                        let request_struct = Some(format!(
                            "{}Request",
                            method_struct_prefix(interface_name, &raw_setter)
                        ));
                        let request_param = ParamContext {
                            name: "value".to_string(),
                            raw_name: "value".to_string(),
                            ty: param.clone(),
                            source: param_source_code(ParamSource::Body),
                            serde_rename: None,
                        };
                        out.push(MethodContext {
                            name: setter.clone(),
                            raw_name: raw_setter.clone(),
                            params: vec![format!("value: {param}")],
                            param_names: vec!["value".to_string()],
                            ret: "()".to_string(),
                            http_method: http_method_code(HttpMethod::Post),
                            http_method_fn: http_method_fn(HttpMethod::Post),
                            reqwest_method: reqwest_method_code(HttpMethod::Post),
                            path: setter_path,
                            struct_prefix: method_struct_prefix(interface_name, &raw_setter),
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            body_params: vec![request_param.clone()],
                            request_ty: request_struct.clone().unwrap_or_else(|| "()".to_string()),
                            request_struct,
                            request_params: vec![request_param],
                        });
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let name = rust_ident(&declarator.0);
                    let raw_name = declarator.0.clone();
                    let ret = attr_return_type(&spec.ty);
                    let path = default_path(module_path, interface_name, &raw_name);
                    let param = render_param_type(&spec.ty, None);
                    let request_struct = None;
                    out.push(MethodContext {
                        name: name.clone(),
                        raw_name: raw_name.clone(),
                        params: Vec::new(),
                        param_names: Vec::new(),
                        ret,
                        http_method: http_method_code(HttpMethod::Get),
                        http_method_fn: http_method_fn(HttpMethod::Get),
                        reqwest_method: reqwest_method_code(HttpMethod::Get),
                        path,
                        struct_prefix: method_struct_prefix(interface_name, &raw_name),
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        body_params: Vec::new(),
                        request_ty: "()".to_string(),
                        request_struct,
                        request_params: Vec::new(),
                    });
                    let raw_setter = format!("set_{raw_name}");
                    let setter = rust_ident(&raw_setter);
                    let setter_path = default_path(module_path, interface_name, &raw_setter);
                    let request_struct = Some(format!(
                        "{}Request",
                        method_struct_prefix(interface_name, &raw_setter)
                    ));
                    let request_param = ParamContext {
                        name: "value".to_string(),
                        raw_name: "value".to_string(),
                        ty: param.clone(),
                        source: param_source_code(ParamSource::Body),
                        serde_rename: None,
                    };
                    out.push(MethodContext {
                        name: setter.clone(),
                        raw_name: raw_setter.clone(),
                        params: vec![format!("value: {param}")],
                        param_names: vec!["value".to_string()],
                        ret: "()".to_string(),
                        http_method: http_method_code(HttpMethod::Post),
                        http_method_fn: http_method_fn(HttpMethod::Post),
                        reqwest_method: reqwest_method_code(HttpMethod::Post),
                        path: setter_path,
                        struct_prefix: method_struct_prefix(interface_name, &raw_setter),
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        body_params: vec![request_param.clone()],
                        request_ty: request_struct.clone().unwrap_or_else(|| "()".to_string()),
                        request_struct,
                        request_params: vec![request_param],
                    });
                }
            }
            out
        }
    }
}

struct AttrNames {
    raw: String,
    rust: String,
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<AttrNames> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![AttrNames {
            raw: decl.0.clone(),
            rust: rust_ident(&decl.0),
        }],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn attr_return_type(ty: &hir::TypeSpec) -> String {
    axum_type(ty)
}

fn render_param_type(ty: &hir::TypeSpec, attr: Option<&hir::ParamAttribute>) -> String {
    let _ = attr;
    axum_type(ty)
}

fn method_struct_prefix(interface_name: &str, method_name: &str) -> String {
    let interface = interface_name.strip_prefix("r#").unwrap_or(interface_name);
    let method = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}",
        interface.to_case(Case::Pascal),
        method.to_case(Case::Pascal)
    )
}

fn axum_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => rust_integer_type(value),
            hir::SimpleTypeSpec::FloatingPtType => "f64".to_string(),
            hir::SimpleTypeSpec::CharType => "char".to_string(),
            hir::SimpleTypeSpec::WideCharType => "char".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::AnyType => "xidl_rust_axum::serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ObjectType => "xidl_rust_axum::serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ValueBaseType => "xidl_rust_axum::serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => render_scoped_name(value),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                format!("Vec<{}>", axum_type(&seq.ty))
            }
            hir::TemplateTypeSpec::StringType(_) => "String".to_string(),
            hir::TemplateTypeSpec::WideStringType(_) => "String".to_string(),
            hir::TemplateTypeSpec::FixedPtType(_) => "f64".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                format!(
                    "BTreeMap<{}, {}>",
                    axum_type(&map.key),
                    axum_type(&map.value)
                )
            }
        },
    }
}

fn render_scoped_name(value: &hir::ScopedName) -> String {
    let mut iter = value.name.iter();
    let mut parts = Vec::new();
    if let Some(first) = iter.next() {
        if !value.is_root && first == "crate" {
            parts.push("crate".to_string());
        } else {
            parts.push(rust_ident(first));
        }
    }
    for part in iter {
        parts.push(rust_ident(part));
    }
    let path = parts.join("::");
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

fn default_path(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    format!("/{}", parts.join("/"))
}

fn route_from_annotations(
    annotations: &[hir::Annotation],
    default_method: HttpMethod,
    default_path: String,
) -> (HttpMethod, String) {
    for annotation in annotations {
        let Some(method) = method_from_annotation(annotation) else {
            continue;
        };
        let mut path = None;
        if let Some(params) = annotation_params(annotation) {
            let params = normalize_params(params);
            path = params.get("path").cloned();
        }
        return (method, path.unwrap_or(default_path));
    }
    (default_method, default_path)
}

fn method_from_annotation(annotation: &hir::Annotation) -> Option<HttpMethod> {
    let name = annotation_name(annotation)?;
    match name.to_ascii_lowercase().as_str() {
        "get" => Some(HttpMethod::Get),
        "post" => Some(HttpMethod::Post),
        "put" => Some(HttpMethod::Put),
        "patch" => Some(HttpMethod::Patch),
        "delete" => Some(HttpMethod::Delete),
        "head" => Some(HttpMethod::Head),
        "options" => Some(HttpMethod::Options),
        _ => None,
    }
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

fn normalize_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match params {
        hir::AnnotationParams::Raw(value) => {
            for (key, value) in parse_raw_params(value) {
                out.insert(key.to_ascii_lowercase(), value);
            }
        }
        hir::AnnotationParams::Params(values) => {
            for value in values {
                let raw = value
                    .value
                    .as_ref()
                    .map(render_const_expr)
                    .unwrap_or_default();
                out.insert(
                    value.ident.to_ascii_lowercase(),
                    trim_quotes(&raw).unwrap_or(raw),
                );
            }
        }
        hir::AnnotationParams::ConstExpr(expr) => {
            let rendered = render_const_expr(expr);
            out.insert(
                "value".to_string(),
                trim_quotes(&rendered).unwrap_or(rendered),
            );
        }
    }
    out
}

fn parse_raw_params(raw: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            buf.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote.is_some() {
            escaped = true;
            buf.push(ch);
            continue;
        }
        match ch {
            '\'' | '"' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                buf.push(ch);
            }
            ',' if quote.is_none() => {
                let item = buf.trim();
                if !item.is_empty() {
                    parts.push(item.to_string());
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }

    let item = buf.trim();
    if !item.is_empty() {
        parts.push(item.to_string());
    }

    let mut out = Vec::new();
    for part in parts {
        if let Some((key, value)) = part.split_once('=') {
            let value = trim_quotes(value.trim()).unwrap_or_else(|| value.trim().to_string());
            out.push((key.trim().to_string(), unescape_param_value(&value)));
        }
    }
    out
}

fn unescape_param_value(value: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        out.push(ch);
    }
    out
}

fn trim_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(value[1..value.len() - 1].to_string());
        }
    }
    None
}

fn render_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(
        expr,
        &crate::generate::rust::util::rust_scoped_name,
        &crate::generate::rust::util::rust_literal,
    )
}

fn serde_rename(raw: &str, rust: &str) -> Option<String> {
    let stripped = rust.strip_prefix("r#").unwrap_or(rust);
    if stripped == raw {
        None
    } else {
        Some(raw.to_string())
    }
}

fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut buf = String::new();
    let mut in_param = false;

    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
            }
            '}' if in_param => {
                if !buf.is_empty() {
                    out.insert(buf.clone());
                }
                in_param = false;
            }
            _ => {
                if in_param {
                    buf.push(ch);
                }
            }
        }
    }

    out
}

fn default_param_source(method: HttpMethod) -> ParamSource {
    match method {
        HttpMethod::Get | HttpMethod::Delete | HttpMethod::Head | HttpMethod::Options => {
            ParamSource::Query
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => ParamSource::Body,
    }
}

fn param_source_code(source: ParamSource) -> String {
    match source {
        ParamSource::Query => "ParamSource::Query".to_string(),
        ParamSource::Body => "ParamSource::Body".to_string(),
        ParamSource::Path => "ParamSource::Path".to_string(),
    }
}

fn http_method_code(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "HttpMethod::Get".to_string(),
        HttpMethod::Post => "HttpMethod::Post".to_string(),
        HttpMethod::Put => "HttpMethod::Put".to_string(),
        HttpMethod::Patch => "HttpMethod::Patch".to_string(),
        HttpMethod::Delete => "HttpMethod::Delete".to_string(),
        HttpMethod::Head => "HttpMethod::Head".to_string(),
        HttpMethod::Options => "HttpMethod::Options".to_string(),
    }
}

fn http_method_fn(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "get".to_string(),
        HttpMethod::Post => "post".to_string(),
        HttpMethod::Put => "put".to_string(),
        HttpMethod::Patch => "patch".to_string(),
        HttpMethod::Delete => "delete".to_string(),
        HttpMethod::Head => "head".to_string(),
        HttpMethod::Options => "options".to_string(),
    }
}

fn reqwest_method_code(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "GET".to_string(),
        HttpMethod::Post => "POST".to_string(),
        HttpMethod::Put => "PUT".to_string(),
        HttpMethod::Patch => "PATCH".to_string(),
        HttpMethod::Delete => "DELETE".to_string(),
        HttpMethod::Head => "HEAD".to_string(),
        HttpMethod::Options => "OPTIONS".to_string(),
    }
}
