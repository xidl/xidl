use crate::error::IdlcResult;
use std::collections::BTreeSet;

use xidl_parser::hir;

use super::definition_names::{go_capitalize, go_export_name};

pub(crate) fn constr_type_name(constr: &hir::ConstrTypeDcl, prefix: &[String]) -> String {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::StructForwardDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::EnumDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::UnionDef(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::UnionForwardDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::BitsetDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::BitmaskDcl(def) => go_export_name(prefix, &def.ident),
    }
}

pub(crate) fn render_const_expr(expr: &hir::ConstExpr) -> IdlcResult<String> {
    Ok(crate::generate::render_const_expr(
        expr,
        &go_scoped_name,
        &go_literal,
    ))
}

pub(crate) fn go_scoped_name(value: &hir::ScopedName) -> String {
    value
        .name
        .iter()
        .map(|part| go_capitalize(part))
        .collect::<Vec<_>>()
        .join("")
}

pub(crate) fn go_literal(value: &hir::Literal) -> String {
    match value {
        hir::Literal::IntegerLiteral(lit) => lit.0.clone(),
        hir::Literal::FloatingPtLiteral(lit) => {
            let sign = lit
                .sign
                .as_ref()
                .map(hir::IntegerSign::as_str)
                .unwrap_or("");
            format!("{}{}.{}", sign, lit.integer.0, lit.fraction.0)
        }
        hir::Literal::CharLiteral(value) => value.clone(),
        hir::Literal::WideCharacterLiteral(value) => {
            value.strip_prefix('L').unwrap_or(value).into()
        }
        hir::Literal::StringLiteral(value) => value.clone(),
        hir::Literal::WideStringLiteral(value) => value.strip_prefix('L').unwrap_or(value).into(),
        hir::Literal::BooleanLiteral(value) => value.to_string(),
    }
}

pub(crate) fn go_const_type(ty: &hir::ConstType) -> String {
    match ty {
        hir::ConstType::IntegerType(value) => go_integer_type(value),
        hir::ConstType::FloatingPtType | hir::ConstType::FixedPtConstType => "float64".to_string(),
        hir::ConstType::CharType | hir::ConstType::WideCharType => "rune".to_string(),
        hir::ConstType::BooleanType => "bool".to_string(),
        hir::ConstType::OctetType => "byte".to_string(),
        hir::ConstType::StringType(_) | hir::ConstType::WideStringType(_) => "string".to_string(),
        hir::ConstType::ScopedName(value) => go_scoped_name(value),
        hir::ConstType::SequenceType(_) => "any".to_string(),
    }
}

pub(crate) fn go_type_with_prefix(
    ty: &hir::TypeSpec,
    current_prefix: &[String],
    versioned: bool,
    type_paths: &BTreeSet<Vec<String>>,
) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => go_integer_type(value),
        hir::TypeSpec::FloatingPtType => "float64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "rune".to_string(),
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "any".to_string()
        }
        hir::TypeSpec::ScopedName(value) => {
            go_scoped_name_with_prefix(value, current_prefix, versioned, type_paths)
        }
        hir::TypeSpec::SequenceType(seq) => {
            format!(
                "[]{}",
                go_type_with_prefix(&seq.ty, current_prefix, versioned, type_paths)
            )
        }
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "string".to_string(),
        hir::TypeSpec::FixedPtType(_) => "float64".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!(
                "map[{}]{}",
                go_type_with_prefix(&map.key, current_prefix, versioned, type_paths),
                go_type_with_prefix(&map.value, current_prefix, versioned, type_paths)
            )
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}[{}]",
            go_capitalize(&value.ident),
            value
                .args
                .iter()
                .map(|value| go_type_with_prefix(value, current_prefix, versioned, type_paths))
                .collect::<Vec<_>>()
                .join(", "),
        ),
    }
}

fn go_scoped_name_with_prefix(
    value: &hir::ScopedName,
    current_prefix: &[String],
    versioned: bool,
    type_paths: &BTreeSet<Vec<String>>,
) -> String {
    if value.name.is_empty() {
        return go_scoped_name(value);
    }
    if !versioned {
        return go_scoped_name(value);
    }
    if let Some(path) = resolve_type_path(value, current_prefix, type_paths) {
        return path
            .iter()
            .map(|part| go_capitalize(part))
            .collect::<Vec<_>>()
            .join("");
    }
    let mut parts = Vec::new();
    if value.name.len() == 1 {
        parts.extend(current_prefix.iter().cloned());
    } else if let Some(version) = current_prefix.first() {
        parts.push(version.clone());
    }
    parts.extend(value.name.iter().cloned());
    parts
        .iter()
        .map(|part| go_capitalize(part))
        .collect::<Vec<_>>()
        .join("")
}

fn resolve_type_path(
    value: &hir::ScopedName,
    current_prefix: &[String],
    type_paths: &BTreeSet<Vec<String>>,
) -> Option<Vec<String>> {
    if value.is_root {
        return type_paths
            .iter()
            .find(|path| path.as_slice() == value.name.as_slice())
            .cloned();
    }
    if value.name.len() == 1 {
        for end in (0..=current_prefix.len()).rev() {
            let mut candidate = current_prefix[..end].to_vec();
            candidate.extend(value.name.iter().cloned());
            if type_paths.contains(&candidate) {
                return Some(candidate);
            }
        }
    } else if let Some(version) = current_prefix.first() {
        let mut candidate = vec![version.clone()];
        candidate.extend(value.name.iter().cloned());
        if type_paths.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn go_integer_type(value: &hir::IntegerType) -> String {
    match value {
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
    }
}
