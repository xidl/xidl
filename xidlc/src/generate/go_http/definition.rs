use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::has_optional_annotation;
use convert_case::Casing;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use xidl_parser::hir;

use super::GoHttpRenderer;
use super::{HttpMethod, ParamDirection, ParamMeta, ParamSource, RouteTemplate, SourceBinding};

#[derive(Serialize)]
struct RequestBindTemplate<'a> {
    field: &'a str,
    call: &'a str,
    ty: &'a str,
    optional_string: bool,
}

#[derive(Serialize)]
struct EncodeTemplate<'a> {
    wire_name: &'a str,
    field: &'a str,
    ty: &'a str,
}

#[derive(Serialize)]
struct ResponseEncodeTemplate<'a> {
    wire_name: &'a str,
    field: &'a str,
    ty: &'a str,
}

#[derive(Serialize)]
struct ResponseHeaderDecodeTemplate<'a> {
    wire_name: &'a str,
    field: &'a str,
    ty: &'a str,
}

#[derive(Serialize)]
struct ResponseCookieDecodeTemplate<'a> {
    wire_name: &'a str,
    field_name: &'a str,
    ty: &'a str,
}

#[derive(Serialize)]
struct FormatPathTemplate<'a> {
    struct_prefix: &'a str,
    request_struct: &'a str,
    raw_path: &'a str,
    trim_query_template: bool,
    replacements: Vec<FormatPathReplacement>,
}

#[derive(Serialize)]
struct FormatPathReplacement {
    replacement: String,
    replacement_catchall: String,
    expr: String,
}

pub(crate) fn param_meta(
    param: &hir::ParamDcl,
    direction: ParamDirection,
    default_source: ParamSource,
    all_path_param_names: &HashSet<String>,
    all_query_template_names: &HashSet<String>,
) -> IdlcResult<ParamMeta> {
    let explicit = explicit_param_binding(param)?;
    let source = if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
        explicit
            .as_ref()
            .map(|value| value.source)
            .unwrap_or(ParamSource::Body)
    } else if let Some(binding) = explicit.as_ref() {
        binding.source
    } else if all_path_param_names.contains(&param.declarator.0) {
        ParamSource::Path
    } else if all_query_template_names.contains(&param.declarator.0) {
        ParamSource::Query
    } else {
        default_source
    };
    let wire_name = explicit
        .as_ref()
        .map(|value| value.bound_name.clone())
        .unwrap_or_else(|| param.declarator.0.clone());
    Ok(ParamMeta {
        field_name: param.declarator.0.to_case(convert_case::Case::Pascal),
        raw_name: param.declarator.0.clone(),
        wire_name,
        ty: go_type(&param.ty),
        optional: has_optional_annotation(&param.annotations),
        source,
    })
}

pub(crate) fn explicit_param_binding(param: &hir::ParamDcl) -> IdlcResult<Option<SourceBinding>> {
    let mut found: Option<SourceBinding> = None;
    for annotation in &param.annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let source = if name.eq_ignore_ascii_case("path") {
            Some(ParamSource::Path)
        } else if name.eq_ignore_ascii_case("query") {
            Some(ParamSource::Query)
        } else if name.eq_ignore_ascii_case("header") {
            Some(ParamSource::Header)
        } else if name.eq_ignore_ascii_case("cookie") {
            Some(ParamSource::Cookie)
        } else {
            None
        };
        let Some(source) = source else {
            continue;
        };
        let bound_name = annotation_params(annotation)
            .map(normalize_params)
            .and_then(|params| params.get("value").cloned())
            .unwrap_or_else(|| param.declarator.0.clone());
        if let Some(existing) = &found {
            if existing.source != source || existing.bound_name != bound_name {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' has conflicting source annotations",
                    param.declarator.0
                )));
            }
        } else {
            found = Some(SourceBinding { source, bound_name });
        }
    }
    Ok(found)
}

pub(crate) fn route_from_annotations(
    annotations: &[hir::Annotation],
    default_method: HttpMethod,
) -> IdlcResult<(HttpMethod, Vec<String>)> {
    let mut method = None;
    let mut paths = Vec::new();
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if let Some(current) = method_from_annotation(annotation) {
            if let Some(previous) = method {
                if previous != current {
                    return Err(IdlcError::rpc(
                        "more than one HTTP verb annotation is not allowed on a method",
                    ));
                }
            }
            method = Some(current);
            if let Some(path) = annotation_params(annotation)
                .map(normalize_params)
                .and_then(|params| params.get("path").cloned())
            {
                paths.push(normalize_path(&path));
            }
            continue;
        }
        if name.eq_ignore_ascii_case("path") {
            if let Some(path) = annotation_params(annotation)
                .map(normalize_params)
                .and_then(|params| params.get("value").cloned())
            {
                paths.push(normalize_path(&path));
            }
        }
    }
    let mut seen = HashSet::new();
    paths.retain(|item| seen.insert(item.clone()));
    Ok((method.unwrap_or(default_method), paths))
}

pub(crate) fn method_from_annotation(annotation: &hir::Annotation) -> Option<HttpMethod> {
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

pub(crate) fn auto_default_method_path(op: &hir::OpDcl, method: HttpMethod) -> IdlcResult<String> {
    let mut path = normalize_path(&op.ident);
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    for param in params {
        if matches!(param_direction(param.attr.as_ref()), ParamDirection::Out) {
            continue;
        }
        let binding = explicit_param_binding(param)?;
        let source = binding
            .as_ref()
            .map(|value| value.source)
            .unwrap_or(default_param_source(method));
        let bound_name = binding
            .as_ref()
            .map(|value| value.bound_name.clone())
            .unwrap_or_else(|| param.declarator.0.clone());
        if matches!(source, ParamSource::Path) {
            path.push('/');
            path.push('{');
            path.push_str(&bound_name);
            path.push('}');
        }
    }
    Ok(path)
}

pub(crate) fn parse_route_template(path: &str) -> IdlcResult<RouteTemplate> {
    let (path, query_params) = split_query_template(path)?;
    let normalized = normalize_path(&path);
    Ok(RouteTemplate {
        path: normalized.clone(),
        path_params: parse_path_params(&normalized),
        query_params,
    })
}

pub(crate) fn split_query_template(path: &str) -> IdlcResult<(String, HashSet<String>)> {
    let mut query_params = HashSet::new();
    if let Some(pos) = path.find("{?") {
        if !path.ends_with('}') {
            return Err(IdlcError::rpc(format!(
                "query template must terminate with '}}' in route '{path}'"
            )));
        }
        let tail = &path[pos + 2..path.len() - 1];
        for name in tail
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            query_params.insert(name.to_string());
        }
        Ok((path[..pos].to_string(), query_params))
    } else {
        Ok((path.to_string(), query_params))
    }
}

pub(crate) fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut in_param = false;
    let mut buf = String::new();
    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
            }
            '}' if in_param => {
                if !buf.is_empty() {
                    out.insert(buf.trim_start_matches('*').to_string());
                }
                in_param = false;
            }
            _ if in_param => buf.push(ch),
            _ => {}
        }
    }
    out
}

pub(crate) fn normalize_path(path: &str) -> String {
    let path = path.trim();
    let with_leading = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut out = String::new();
    let mut prev_slash = false;
    for ch in with_leading.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            prev_slash = false;
            out.push(ch);
        }
    }
    if out.len() > 1 && out.ends_with('/') {
        out.pop();
    }
    out
}

pub(crate) fn default_param_source(method: HttpMethod) -> ParamSource {
    match method {
        HttpMethod::Get | HttpMethod::Delete | HttpMethod::Head | HttpMethod::Options => {
            ParamSource::Query
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => ParamSource::Body,
    }
}

pub(crate) fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

pub(crate) fn go_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => match value {
                hir::IntegerType::Char => "int8".to_string(),
                hir::IntegerType::UChar | hir::IntegerType::U8 => "uint8".to_string(),
                hir::IntegerType::U16 => "uint16".to_string(),
                hir::IntegerType::U32 => "uint32".to_string(),
                hir::IntegerType::U64 => "uint64".to_string(),
                hir::IntegerType::I8 => "int8".to_string(),
                hir::IntegerType::I16 => "int16".to_string(),
                hir::IntegerType::I32 => "int32".to_string(),
                hir::IntegerType::I64 => "int64".to_string(),
            },
            hir::SimpleTypeSpec::FloatingPtType => "float64".to_string(),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => "rune".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => "any".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => value
                .name
                .iter()
                .map(|part| part.to_case(convert_case::Case::Pascal))
                .collect::<Vec<_>>()
                .join(""),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => format!("[]{}", go_type(&seq.ty)),
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                "string".to_string()
            }
            hir::TemplateTypeSpec::FixedPtType(_) => "float64".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                format!("map[{}]{}", go_type(&map.key), go_type(&map.value))
            }
            hir::TemplateTypeSpec::TemplateType(value) => {
                value.ident.to_case(convert_case::Case::Pascal)
            }
        },
    }
}

pub(crate) fn export_name(prefix: &[String], value: &str) -> String {
    prefix
        .iter()
        .chain(std::iter::once(&value.to_string()))
        .map(|item| item.to_case(convert_case::Case::Pascal))
        .collect::<Vec<_>>()
        .join("")
}

pub(crate) fn http_method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

pub(crate) fn go_pattern_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'*') {
            chars.next();
            out.push('{');
            for token in chars.by_ref() {
                if token == '}' {
                    out.push_str("...}");
                    break;
                }
                out.push(token);
            }
            continue;
        }

        out.push(ch);
    }

    out
}

pub(crate) fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

pub(crate) fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

pub(crate) fn normalize_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
    crate::generate::utils::http::normalize_annotation_params(params)
}

pub(crate) fn find_basic_realm(annotations: &[hir::Annotation]) -> Option<String> {
    annotations.iter().find_map(|annotation| {
        if !annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("http_basic"))
            .unwrap_or(false)
        {
            return None;
        }
        annotation_params(annotation)
            .map(normalize_params)
            .and_then(|params| params.get("realm").cloned())
    })
}

pub(crate) fn emit_request_bind(
    out: &mut String,
    source_expr: &str,
    param: &ParamMeta,
    source_kind: &str,
) -> IdlcResult<()> {
    let field = format!("req.{}", param.field_name);
    let wire = &param.wire_name;
    let call = match source_kind {
        "Path" => format!("xidlgohttp.PathString({source_expr}, {wire:?})"),
        "Query" => {
            format!("xidlgohttp.QueryString({source_expr}, {wire:?})")
        }
        "Header" => format!("xidlgohttp.HeaderString({source_expr}, {wire:?})"),
        "Cookie" => format!("xidlgohttp.CookieString({source_expr}, {wire:?})"),
        _ => return Ok(()),
    };
    if param.ty != "string" && param.ty != "uint32" && param.ty != "int32" && param.ty != "bool" {
        return Ok(());
    }
    let renderer = GoHttpRenderer::new()?;
    out.push_str(&renderer.render_template(
        "request_bind.go.j2",
        &RequestBindTemplate {
            field: &field,
            call: &call,
            ty: &param.ty,
            optional_string: source_kind == "Query" && param.optional && param.ty == "string",
        },
    )?);
    out.push('\n');
    Ok(())
}

pub(crate) fn emit_query_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    let field = format!("req.{}", param.field_name);
    match param.ty.as_str() {
        "string" | "*string" | "uint32" | "int32" | "bool" => {
            let renderer = GoHttpRenderer::new()?;
            out.push_str(&renderer.render_template(
                "query_encode.go.j2",
                &EncodeTemplate {
                    wire_name: &param.wire_name,
                    field: &field,
                    ty: &param.ty,
                },
            )?);
            out.push('\n');
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn emit_header_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    let field = format!("req.{}", param.field_name);
    match param.ty.as_str() {
        "string" | "uint32" => {
            let renderer = GoHttpRenderer::new()?;
            out.push_str(&renderer.render_template(
                "header_encode.go.j2",
                &EncodeTemplate {
                    wire_name: &param.wire_name,
                    field: &field,
                    ty: &param.ty,
                },
            )?);
            out.push('\n');
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn emit_cookie_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    let field = format!("req.{}", param.field_name);
    if param.ty == "string" {
        let renderer = GoHttpRenderer::new()?;
        out.push_str(&renderer.render_template(
            "cookie_encode.go.j2",
            &EncodeTemplate {
                wire_name: &param.wire_name,
                field: &field,
                ty: &param.ty,
            },
        )?);
        out.push('\n');
    }
    Ok(())
}

pub(crate) fn emit_response_header_encode(
    out: &mut String,
    param: &ParamMeta,
    value: &str,
) -> IdlcResult<()> {
    let field = format!("{value}.{}", param.field_name);
    match param.ty.as_str() {
        "string" | "uint32" => {
            let renderer = GoHttpRenderer::new()?;
            out.push_str(&renderer.render_template(
                "response_header_encode.go.j2",
                &ResponseEncodeTemplate {
                    wire_name: &param.wire_name,
                    field: &field,
                    ty: &param.ty,
                },
            )?);
            out.push('\n');
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn emit_response_cookie_encode(
    out: &mut String,
    param: &ParamMeta,
    value: &str,
) -> IdlcResult<()> {
    let field = format!("{value}.{}", param.field_name);
    if param.ty == "string" {
        let renderer = GoHttpRenderer::new()?;
        out.push_str(&renderer.render_template(
            "response_cookie_encode.go.j2",
            &ResponseEncodeTemplate {
                wire_name: &param.wire_name,
                field: &field,
                ty: &param.ty,
            },
        )?);
        out.push('\n');
    }
    Ok(())
}

pub(crate) fn emit_response_header_decode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    let field = format!("out.{}", param.field_name);
    if param.ty == "string" || param.ty == "uint32" {
        let renderer = GoHttpRenderer::new()?;
        out.push_str(&renderer.render_template(
            "response_header_decode.go.j2",
            &ResponseHeaderDecodeTemplate {
                wire_name: &param.wire_name,
                field: &field,
                ty: &param.ty,
            },
        )?);
        out.push('\n');
    }
    Ok(())
}

pub(crate) fn emit_response_cookie_decode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    if param.ty == "string" {
        let renderer = GoHttpRenderer::new()?;
        out.push_str(&renderer.render_template(
            "response_cookie_decode.go.j2",
            &ResponseCookieDecodeTemplate {
                wire_name: &param.wire_name,
                field_name: &param.field_name,
                ty: &param.ty,
            },
        )?);
        out.push('\n');
    }
    Ok(())
}

pub(crate) fn render_format_path_fn(
    out: &mut String,
    method: &super::MethodMeta,
) -> IdlcResult<()> {
    let raw = method
        .paths
        .first()
        .cloned()
        .unwrap_or_else(|| "/".to_string());
    let mut replacements = Vec::new();
    for param in &method.path_params {
        let replacement = format!("{{{}}}", param.wire_name);
        let replacement_catchall = format!("{{*{}}}", param.wire_name);
        let field = format!("req.{}", param.field_name);
        let expr = if param.ty == "string" {
            field
        } else if param.ty == "uint32" {
            format!("xidlgohttp.FormatUint32({field})")
        } else {
            field
        };
        replacements.push(FormatPathReplacement {
            replacement,
            replacement_catchall,
            expr,
        });
    }
    let renderer = GoHttpRenderer::new()?;
    out.push_str(&renderer.render_template(
        "format_path.go.j2",
        &FormatPathTemplate {
            struct_prefix: &method.struct_prefix,
            request_struct: &method.request_struct,
            raw_path: &raw,
            trim_query_template: raw.contains("{?"),
            replacements,
        },
    )?);
    Ok(())
}

pub(crate) fn strip_interfaces(spec: hir::Specification) -> hir::Specification {
    fn strip_defs(defs: Vec<hir::Definition>) -> Vec<hir::Definition> {
        let mut out = Vec::new();
        for def in defs {
            match def {
                hir::Definition::InterfaceDcl(_) => {}
                hir::Definition::ModuleDcl(mut module) => {
                    module.definition = strip_defs(module.definition);
                    if !module.definition.is_empty() {
                        out.push(hir::Definition::ModuleDcl(module));
                    }
                }
                other => out.push(other),
            }
        }
        out
    }

    hir::Specification(strip_defs(spec.0))
}
