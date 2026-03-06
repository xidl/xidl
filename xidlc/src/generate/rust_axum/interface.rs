use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::{RustAxumRenderOutput, RustAxumRenderer};
use convert_case::{Case, Casing};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use xidl_parser::hir;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamSource {
    Path,
    Query,
    Body,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamDirection {
    In,
    Out,
    InOut,
}

#[derive(Serialize)]
struct MethodContext {
    name: String,
    raw_name: String,
    params: Vec<String>,
    param_names: Vec<String>,
    ret: String,
    response_ty: String,
    http_method: String,
    http_method_fn: String,
    reqwest_method: String,
    path: String,
    paths: Vec<String>,
    struct_prefix: String,
    path_params: Vec<ParamContext>,
    query_params: Vec<ParamContext>,
    body_params: Vec<ParamContext>,
    request_ty: String,
    request_struct: Option<String>,
    request_params: Vec<ParamContext>,
    response_struct: Option<String>,
    response_params: Vec<ParamContext>,
    response_include_return: bool,
    response_is_empty: bool,
    return_is_unit: bool,
}

#[derive(Serialize, Clone)]
struct ParamContext {
    name: String,
    raw_name: String,
    wire_name: String,
    path_template_name: String,
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
                    methods.push(render_op(op, &def.header.ident, module_path)?);
                }
                hir::Export::AttrDcl(attr) => {
                    methods.extend(render_attr(attr, &def.header.ident, module_path));
                }
                _ => {}
            }
        }
    }

    let mut route_bindings = HashMap::new();
    for method in &methods {
        for path in &method.paths {
            let key = format!("{} {}", method.reqwest_method, path);
            if let Some(previous) = route_bindings.insert(key.clone(), method.raw_name.clone()) {
                return Err(IdlcError::rpc(format!(
                    "duplicate HTTP route binding: {key} (methods: {previous}, {})",
                    method.raw_name
                )));
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

fn render_op(
    op: &hir::OpDcl,
    interface_name: &str,
    _module_path: &[String],
) -> IdlcResult<MethodContext> {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => axum_type(ty),
    };
    let return_is_unit = matches!(&op.ty, hir::OpTypeSpec::Void);
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
    let mut response_params = Vec::new();

    let (method, mut paths) = route_from_annotations(&op.annotations, HttpMethod::Post)?;
    if paths.is_empty() {
        paths.push(auto_default_method_path(op, method)?);
    }
    validate_head_constraints(op, method)?;
    let path = paths
        .first()
        .cloned()
        .unwrap_or_else(|| format!("/{}", op.ident));
    let path_param_sets = paths
        .iter()
        .map(|item| parse_path_params(item))
        .collect::<Vec<_>>();
    let all_path_param_names: HashSet<String> = path_param_sets
        .iter()
        .flat_map(|set| set.iter().cloned())
        .collect();
    let default_source = default_param_source(method);
    let mut path_binding_count = HashMap::<String, usize>::new();

    for param in params {
        let direction = param_direction(param.attr.as_ref());
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        if let Some(binding) = explicit_param_binding(param)? {
            if matches!(binding.source, ParamSource::Path)
                && !all_path_param_names.contains(&binding.bound_name)
            {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' is annotated with @path but '{}' is not present in any route template of method '{}'",
                    param.declarator.0, binding.bound_name, op.ident
                )));
            }
        }
    }

    for param in params {
        let ty = render_param_type(&param.ty, param.attr.as_ref());
        let name = rust_ident(&param.declarator.0);
        let direction = param_direction(param.attr.as_ref());
        if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
            response_params.push(ParamContext {
                name: name.clone(),
                raw_name: param.declarator.0.clone(),
                wire_name: param.declarator.0.clone(),
                path_template_name: String::new(),
                ty: ty.clone(),
                source: String::new(),
                serde_rename: serde_rename(&param.declarator.0, &name),
            });
        }
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        param_list.push(format!("{name}: {ty}"));
        param_names.push(name.clone());
        let binding = explicit_param_binding(param)?;
        let (source, wire_name) = match binding {
            Some(binding) => (binding.source, binding.bound_name),
            None if all_path_param_names.contains(&param.declarator.0) => {
                (ParamSource::Path, param.declarator.0.clone())
            }
            None => (default_source, param.declarator.0.clone()),
        };
        if matches!(source, ParamSource::Path)
            && !path_name_in_all_routes(&wire_name, &path_param_sets)
        {
            return Err(IdlcError::rpc(format!(
                "parameter '{}' is bound to path variable '{}' but it is not present in every route template of method '{}'",
                param.declarator.0, wire_name, op.ident
            )));
        }
        let serde_name = if matches!(source, ParamSource::Body) {
            param.declarator.0.clone()
        } else {
            wire_name.clone()
        };
        let ctx = ParamContext {
            name: name.clone(),
            raw_name: param.declarator.0.clone(),
            path_template_name: if matches!(source, ParamSource::Path)
                && path_param_is_catch_all(&path, &wire_name)
            {
                format!("*{wire_name}")
            } else {
                wire_name.clone()
            },
            wire_name,
            ty,
            source: param_source_code(source),
            serde_rename: serde_rename(&serde_name, &name),
        };
        request_params.push(ctx.clone());
        match source {
            ParamSource::Path => {
                *path_binding_count.entry(ctx.wire_name.clone()).or_insert(0) += 1;
                path_params.push(ctx);
            }
            ParamSource::Query => query_params.push(ctx),
            ParamSource::Body => body_params.push(ctx),
        }
    }
    for route_params in &path_param_sets {
        for route_param in route_params {
            match path_binding_count.get(route_param).copied().unwrap_or(0) {
                0 => {
                    return Err(IdlcError::rpc(format!(
                        "route template variable '{}' has no matching request-side path parameter in method '{}'",
                        route_param, op.ident
                    )));
                }
                1 => {}
                _ => {
                    return Err(IdlcError::rpc(format!(
                        "route template variable '{}' is bound by multiple request-side path parameters in method '{}'",
                        route_param, op.ident
                    )));
                }
            }
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
    let response_output_count = usize::from(!return_is_unit) + response_params.len();
    let response_is_empty = response_output_count == 0;
    let response_include_return = !return_is_unit;
    let response_struct = if response_output_count > 1 {
        Some(format!(
            "{}Response",
            method_struct_prefix(interface_name, &op.ident)
        ))
    } else {
        None
    };
    let response_ty = if let Some(response_struct) = &response_struct {
        response_struct.clone()
    } else if !return_is_unit {
        ret.clone()
    } else if let Some(param) = response_params.first() {
        param.ty.clone()
    } else {
        "()".to_string()
    };
    Ok(MethodContext {
        name: method_name,
        raw_name: op.ident.clone(),
        params: param_list,
        param_names,
        ret,
        response_ty,
        http_method: http_method_code(method),
        http_method_fn: http_method_fn(method),
        reqwest_method: reqwest_method_code(method),
        path,
        paths,
        struct_prefix: method_struct_prefix(interface_name, &op.ident),
        path_params,
        query_params,
        body_params,
        request_ty,
        request_struct,
        request_params,
        response_struct,
        response_params,
        response_include_return,
        response_is_empty,
        return_is_unit,
    })
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
                    ret: ret.clone(),
                    response_ty: ret,
                    http_method: http_method_code(HttpMethod::Get),
                    http_method_fn: http_method_fn(HttpMethod::Get),
                    reqwest_method: reqwest_method_code(HttpMethod::Get),
                    paths: vec![path.clone()],
                    path,
                    struct_prefix: method_struct_prefix(interface_name, &raw),
                    path_params: Vec::new(),
                    query_params: Vec::new(),
                    body_params: Vec::new(),
                    request_ty: "()".to_string(),
                    request_struct,
                    request_params: Vec::new(),
                    response_struct: None,
                    response_params: Vec::new(),
                    response_include_return: true,
                    response_is_empty: false,
                    return_is_unit: false,
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
                            ret: ret.clone(),
                            response_ty: ret,
                            http_method: http_method_code(HttpMethod::Get),
                            http_method_fn: http_method_fn(HttpMethod::Get),
                            reqwest_method: reqwest_method_code(HttpMethod::Get),
                            paths: vec![path.clone()],
                            path,
                            struct_prefix: method_struct_prefix(interface_name, &raw_name),
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            body_params: Vec::new(),
                            request_ty: "()".to_string(),
                            request_struct,
                            request_params: Vec::new(),
                            response_struct: None,
                            response_params: Vec::new(),
                            response_include_return: true,
                            response_is_empty: false,
                            return_is_unit: false,
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
                            wire_name: "value".to_string(),
                            path_template_name: String::new(),
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
                            response_ty: "()".to_string(),
                            http_method: http_method_code(HttpMethod::Post),
                            http_method_fn: http_method_fn(HttpMethod::Post),
                            reqwest_method: reqwest_method_code(HttpMethod::Post),
                            paths: vec![setter_path.clone()],
                            path: setter_path,
                            struct_prefix: method_struct_prefix(interface_name, &raw_setter),
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            body_params: vec![request_param.clone()],
                            request_ty: request_struct.clone().unwrap_or_else(|| "()".to_string()),
                            request_struct,
                            request_params: vec![request_param],
                            response_struct: None,
                            response_params: Vec::new(),
                            response_include_return: false,
                            response_is_empty: true,
                            return_is_unit: true,
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
                        ret: ret.clone(),
                        response_ty: ret,
                        http_method: http_method_code(HttpMethod::Get),
                        http_method_fn: http_method_fn(HttpMethod::Get),
                        reqwest_method: reqwest_method_code(HttpMethod::Get),
                        paths: vec![path.clone()],
                        path,
                        struct_prefix: method_struct_prefix(interface_name, &raw_name),
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        body_params: Vec::new(),
                        request_ty: "()".to_string(),
                        request_struct,
                        request_params: Vec::new(),
                        response_struct: None,
                        response_params: Vec::new(),
                        response_include_return: true,
                        response_is_empty: false,
                        return_is_unit: false,
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
                        wire_name: "value".to_string(),
                        path_template_name: String::new(),
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
                        response_ty: "()".to_string(),
                        http_method: http_method_code(HttpMethod::Post),
                        http_method_fn: http_method_fn(HttpMethod::Post),
                        reqwest_method: reqwest_method_code(HttpMethod::Post),
                        paths: vec![setter_path.clone()],
                        path: setter_path,
                        struct_prefix: method_struct_prefix(interface_name, &raw_setter),
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        body_params: vec![request_param.clone()],
                        request_ty: request_struct.clone().unwrap_or_else(|| "()".to_string()),
                        request_struct,
                        request_params: vec![request_param],
                        response_struct: None,
                        response_params: Vec::new(),
                        response_include_return: false,
                        response_is_empty: true,
                        return_is_unit: true,
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

fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
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
            hir::TemplateTypeSpec::TemplateType(value) => format!(
                "{}<{}>",
                rust_ident(&value.ident),
                value
                    .args
                    .iter()
                    .map(axum_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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
) -> IdlcResult<(HttpMethod, Vec<String>)> {
    let mut verb_method = None;
    let mut paths = Vec::new();

    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if let Some(method) = method_from_annotation(annotation) {
            if let Some(prev) = verb_method {
                if prev != method {
                    return Err(IdlcError::rpc(
                        "more than one HTTP verb annotation is not allowed on a method",
                    ));
                }
            }
            verb_method = Some(method);
            if let Some(params) = annotation_params(annotation) {
                let params = normalize_params(params);
                if let Some(path) = params.get("path") {
                    paths.push(normalize_path(path));
                }
            }
            continue;
        }

        if name.eq_ignore_ascii_case("path") {
            if let Some(params) = annotation_params(annotation) {
                let params = normalize_params(params);
                if let Some(path) = params.get("value").or_else(|| params.get("path")) {
                    paths.push(normalize_path(path));
                }
            }
        }
    }

    let mut dedup = HashSet::new();
    paths.retain(|path| dedup.insert(path.clone()));
    for path in &paths {
        validate_route_template(path)?;
    }
    Ok((verb_method.unwrap_or(default_method), paths))
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

struct SourceBinding {
    source: ParamSource,
    bound_name: String,
}

fn explicit_param_binding(param: &hir::ParamDcl) -> IdlcResult<Option<SourceBinding>> {
    let mut found = None;
    for annotation in &param.annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("path") {
            Some(ParamSource::Path)
        } else if name.eq_ignore_ascii_case("query") {
            Some(ParamSource::Query)
        } else {
            None
        };
        let Some(current) = current else {
            continue;
        };
        let bound_name = annotation_params(annotation)
            .map(normalize_params)
            .and_then(|params| params.get("value").cloned())
            .unwrap_or_else(|| param.declarator.0.clone());
        match found {
            None => {
                found = Some(SourceBinding {
                    source: current,
                    bound_name,
                })
            }
            Some(ref prev) if prev.source == current && prev.bound_name == bound_name => {}
            Some(_) => {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' has conflicting source annotations (@path/@query)",
                    param.declarator.0
                )));
            }
        }
    }
    Ok(found)
}

fn auto_default_method_path(op: &hir::OpDcl, method: HttpMethod) -> IdlcResult<String> {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let default_source = default_param_source(method);
    let mut path = normalize_path(&op.ident);
    for param in params {
        if matches!(param_direction(param.attr.as_ref()), ParamDirection::Out) {
            continue;
        }
        let binding = explicit_param_binding(param)?;
        let (source, bound_name) = match binding {
            Some(binding) => (binding.source, binding.bound_name),
            None => (default_source, param.declarator.0.clone()),
        };
        if matches!(source, ParamSource::Path) {
            path.push('/');
            path.push('{');
            path.push_str(&bound_name);
            path.push('}');
        }
    }
    Ok(path)
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

fn normalize_path(path: &str) -> String {
    let path = path.trim();
    let with_leading = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut collapsed = String::with_capacity(with_leading.len());
    let mut prev_slash = false;
    for ch in with_leading.chars() {
        if ch == '/' {
            if !prev_slash {
                collapsed.push(ch);
            }
            prev_slash = true;
        } else {
            collapsed.push(ch);
            prev_slash = false;
        }
    }
    if collapsed.len() > 1 && collapsed.ends_with('/') {
        collapsed.pop();
    }
    if collapsed.is_empty() {
        "/".to_string()
    } else {
        collapsed
    }
}

fn path_name_in_all_routes(name: &str, route_sets: &[HashSet<String>]) -> bool {
    route_sets.iter().all(|set| set.contains(name))
}

fn validate_head_constraints(op: &hir::OpDcl, method: HttpMethod) -> IdlcResult<()> {
    if !matches!(method, HttpMethod::Head) {
        return Ok(());
    }
    if !matches!(op.ty, hir::OpTypeSpec::Void) {
        return Err(IdlcError::rpc(format!(
            "HEAD method '{}' must return void",
            op.ident
        )));
    }
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    for param in params {
        if matches!(
            param_direction(param.attr.as_ref()),
            ParamDirection::Out | ParamDirection::InOut
        ) {
            return Err(IdlcError::rpc(format!(
                "HEAD method '{}' cannot contain out/inout parameter '{}'",
                op.ident, param.declarator.0
            )));
        }
    }
    Ok(())
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
                    out.insert(strip_path_param_prefix(&buf));
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

fn strip_path_param_prefix(value: &str) -> String {
    value.strip_prefix('*').unwrap_or(value).to_string()
}

fn path_param_is_catch_all(path: &str, name: &str) -> bool {
    path.contains(&format!("{{*{name}}}"))
}

fn validate_route_template(path: &str) -> IdlcResult<()> {
    let mut start = 0usize;
    let mut catch_all_count = 0usize;
    while let Some(open_rel) = path[start..].find('{') {
        let open = start + open_rel;
        let close = path[open + 1..]
            .find('}')
            .map(|value| open + 1 + value)
            .ok_or_else(|| {
                IdlcError::rpc(format!("route template has unmatched '{{' in '{path}'"))
            })?;
        let token = &path[open + 1..close];
        let is_catch_all = token.starts_with('*');
        let name = token.strip_prefix('*').unwrap_or(token);
        if name.is_empty() {
            return Err(IdlcError::rpc(format!(
                "route template has empty path variable in '{path}'"
            )));
        }
        if is_catch_all {
            catch_all_count += 1;
            if catch_all_count > 1 {
                return Err(IdlcError::rpc(format!(
                    "route template contains more than one catch-all variable: '{path}'"
                )));
            }
            if close + 1 != path.len() {
                return Err(IdlcError::rpc(format!(
                    "catch-all variable must be at the end of route template: '{path}'"
                )));
            }
        }
        start = close + 1;
    }
    Ok(())
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
