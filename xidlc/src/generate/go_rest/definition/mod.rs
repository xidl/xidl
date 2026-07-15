mod definition_emit;
mod definition_format;
mod definition_templates;

use convert_case::Casing;
use xidl_parser::hir;

use super::HttpMethod;

pub(crate) use self::definition_emit::{
    emit_cookie_encode, emit_header_encode, emit_query_encode, emit_request_bind,
    emit_response_cookie_decode, emit_response_cookie_encode, emit_response_header_decode,
    emit_response_header_encode,
};
pub(crate) use self::definition_format::{render_format_path_fn, strip_interfaces};

pub(crate) fn go_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => match value {
            hir::IntegerType::Char => "int8".to_string(),
            hir::IntegerType::UChar | hir::IntegerType::Octet | hir::IntegerType::U8 => {
                "uint8".to_string()
            }
            hir::IntegerType::U16 => "uint16".to_string(),
            hir::IntegerType::U32 => "uint32".to_string(),
            hir::IntegerType::U64 => "uint64".to_string(),
            hir::IntegerType::I8 => "int8".to_string(),
            hir::IntegerType::I16 => "int16".to_string(),
            hir::IntegerType::I32 => "int32".to_string(),
            hir::IntegerType::I64 => "int64".to_string(),
        },
        hir::TypeSpec::FloatingPtType => "float64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "rune".to_string(),
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "any".to_string()
        }
        hir::TypeSpec::ScopedName(value) => value
            .name
            .iter()
            .map(|part| part.to_case(convert_case::Case::Pascal))
            .collect::<Vec<_>>()
            .join(""),
        hir::TypeSpec::SequenceType(seq) => format!("[]{}", go_type(&seq.ty)),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "string".to_string(),
        hir::TypeSpec::FixedPtType(_) => "float64".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!("map[{}]{}", go_type(&map.key), go_type(&map.value))
        }
        hir::TypeSpec::TemplateType(value) => value.ident.to_case(convert_case::Case::Pascal),
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
        if ch == '{' {
            if chars.peek() == Some(&'*') {
                chars.next();
                out.push('*');
                for token in chars.by_ref() {
                    if token == '}' {
                        break;
                    }
                    out.push(token);
                }
            } else {
                out.push(':');
                for token in chars.by_ref() {
                    if token == '}' {
                        break;
                    }
                    out.push(token);
                }
            }
            continue;
        }

        out.push(ch);
    }

    out
}
