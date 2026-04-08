use crate::error::IdlcResult;
use convert_case::Casing;
use serde::Serialize;
use xidl_parser::hir;

use super::GoHttpRenderer;
use super::{HttpMethod, ParamMeta};

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
