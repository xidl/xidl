//! Deterministic rendering of HIR type and HTTP enum values.

use crate::hir::{self, TypeSpec};
use crate::rest_hir::{HttpMethod, HttpParamKind};

pub(super) fn render_switch_type(value: &hir::SwitchTypeSpec) -> String {
    match value {
        hir::SwitchTypeSpec::IntegerType(value) => render_integer_type(value),
        hir::SwitchTypeSpec::CharType => "char".to_string(),
        hir::SwitchTypeSpec::WideCharType => "wchar".to_string(),
        hir::SwitchTypeSpec::BooleanType => "boolean".to_string(),
        hir::SwitchTypeSpec::ScopedName(value) => render_scoped_name(value),
        hir::SwitchTypeSpec::OctetType => "octet".to_string(),
    }
}

fn render_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "char",
        hir::IntegerType::UChar => "unsigned char",
        hir::IntegerType::Octet => "octet",
        hir::IntegerType::U8 => "uint8",
        hir::IntegerType::U16 => "unsigned short",
        hir::IntegerType::U32 => "unsigned long",
        hir::IntegerType::U64 => "unsigned long long",
        hir::IntegerType::I8 => "int8",
        hir::IntegerType::I16 => "short",
        hir::IntegerType::I32 => "long",
        hir::IntegerType::I64 => "long long",
    }
    .to_string()
}

pub(super) fn render_bitfield_type(value: &hir::BitFieldType) -> String {
    match value {
        hir::BitFieldType::Bool => "boolean",
        hir::BitFieldType::Octec => "octet",
        hir::BitFieldType::SignedInt => "signed int",
        hir::BitFieldType::UnsignedInt => "unsigned int",
    }
    .to_string()
}

pub(super) fn render_scoped_name(value: &hir::ScopedName) -> String {
    let prefix = if value.is_root { "::" } else { "" };
    format!("{prefix}{}", value.name.join("::"))
}
pub(super) fn render_declarator(value: &hir::Declarator) -> String {
    match value {
        hir::Declarator::SimpleDeclarator(value) => value.0.clone(),
        hir::Declarator::ArrayDeclarator(value) => value.ident.clone(),
    }
}

pub(super) fn render_element_type(value: &hir::ElementSpecTy) -> String {
    match value {
        hir::ElementSpecTy::TypeSpec(value) => render_type(value),
        hir::ElementSpecTy::ConstrTypeDcl(value) => value.ident().to_string(),
    }
}

pub(super) fn render_type(value: &TypeSpec) -> String {
    match value {
        TypeSpec::IntegerType(value) => match value {
            hir::IntegerType::Char => "char",
            hir::IntegerType::UChar | hir::IntegerType::Octet | hir::IntegerType::U8 => "uint8",
            hir::IntegerType::U16 => "unsigned short",
            hir::IntegerType::U32 => "unsigned long",
            hir::IntegerType::U64 => "unsigned long long",
            hir::IntegerType::I8 => "int8",
            hir::IntegerType::I16 => "short",
            hir::IntegerType::I32 => "long",
            hir::IntegerType::I64 => "long long",
        }
        .to_string(),
        TypeSpec::FloatingPtType => "double".to_string(),
        TypeSpec::CharType => "char".to_string(),
        TypeSpec::WideCharType => "wchar".to_string(),
        TypeSpec::Boolean => "boolean".to_string(),
        TypeSpec::AnyType => "any".to_string(),
        TypeSpec::ObjectType => "Object".to_string(),
        TypeSpec::ValueBaseType => "ValueBase".to_string(),
        TypeSpec::ScopedName(value) => value.name.join("::"),
        TypeSpec::SequenceType(value) => format!("sequence<{}>", render_type(&value.ty)),
        TypeSpec::StringType(_) => "string".to_string(),
        TypeSpec::WideStringType(_) => "wstring".to_string(),
        TypeSpec::FixedPtType(_) => "fixed".to_string(),
        TypeSpec::MapType(value) => format!(
            "map<{}, {}>",
            render_type(&value.key),
            render_type(&value.value)
        ),
        TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            value.ident,
            value
                .args
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub(super) fn render_param_kind(kind: HttpParamKind) -> &'static str {
    match kind {
        HttpParamKind::Path => "path",
        HttpParamKind::Query => "query",
        HttpParamKind::Header => "header",
        HttpParamKind::Cookie => "cookie",
        HttpParamKind::Body => "body",
    }
}

pub(super) fn render_method(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Patch => "patch",
        HttpMethod::Delete => "delete",
        HttpMethod::Head => "head",
        HttpMethod::Options => "options",
    }
    .to_string()
}

pub(super) fn join(parent: &str, ident: &str) -> String {
    if parent.is_empty() {
        ident.to_string()
    } else {
        format!("{parent}::{ident}")
    }
}
