use crate::error::IdlcResult;
use convert_case::{Case, Casing};
use xidl_parser::hir;

use super::definition_names::go_export_name;

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

pub(crate) fn type_with_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> String {
    let base = go_type(ty);
    match decl {
        hir::Declarator::SimpleDeclarator(_) => base,
        hir::Declarator::ArrayDeclarator(value) => {
            let mut out = base;
            for len in value.len.iter().rev() {
                out = format!(
                    "[{}]{}",
                    render_const_expr(&len.0).unwrap_or_else(|_| "0".to_string()),
                    out
                );
            }
            out
        }
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
        .map(|part| part.to_case(Case::Pascal))
        .collect::<Vec<_>>()
        .join("")
}

pub(crate) fn go_literal(value: &hir::Literal) -> String {
    match value {
        hir::Literal::IntegerLiteral(lit) => match lit {
            hir::IntegerLiteral::BinNumber(value)
            | hir::IntegerLiteral::OctNumber(value)
            | hir::IntegerLiteral::DecNumber(value)
            | hir::IntegerLiteral::HexNumber(value) => value.clone(),
        },
        hir::Literal::FloatingPtLiteral(lit) => {
            let sign = lit
                .sign
                .as_ref()
                .map(|value| value.0.as_str())
                .unwrap_or("");
            format!("{}{}.{}", sign, lit.integer.0, lit.fraction.0)
        }
        hir::Literal::CharLiteral(value) => value.clone(),
        hir::Literal::WideCharacterLiteral(value) => {
            value.strip_prefix('L').unwrap_or(value).into()
        }
        hir::Literal::StringLiteral(value) => value.clone(),
        hir::Literal::WideStringLiteral(value) => value.strip_prefix('L').unwrap_or(value).into(),
        hir::Literal::BooleanLiteral(value) => value.to_ascii_lowercase(),
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

pub(crate) fn go_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => go_integer_type(value),
            hir::SimpleTypeSpec::FloatingPtType => "float64".to_string(),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => "rune".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => "any".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => go_scoped_name(value),
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
            hir::TemplateTypeSpec::TemplateType(value) => format!(
                "{}[{}]",
                value.ident.to_case(Case::Pascal),
                value
                    .args
                    .iter()
                    .map(go_type)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        },
    }
}

fn go_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "int8".to_string(),
        hir::IntegerType::UChar | hir::IntegerType::U8 => "uint8".to_string(),
        hir::IntegerType::U16 => "uint16".to_string(),
        hir::IntegerType::U32 => "uint32".to_string(),
        hir::IntegerType::U64 => "uint64".to_string(),
        hir::IntegerType::I8 => "int8".to_string(),
        hir::IntegerType::I16 => "int16".to_string(),
        hir::IntegerType::I32 => "int32".to_string(),
        hir::IntegerType::I64 => "int64".to_string(),
    }
}
