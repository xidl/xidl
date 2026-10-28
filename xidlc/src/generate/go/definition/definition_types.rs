use crate::error::IdlcResult;

use xidl_parser::hir;

use super::super::GoPackages;
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

pub(crate) fn type_with_decl(
    ty: &hir::TypeSpec,
    decl: &hir::Declarator,
    packages: &GoPackages,
) -> String {
    let base = go_type(ty, packages);
    match decl {
        hir::Declarator::SimpleDeclarator(_) => base,
        hir::Declarator::ArrayDeclarator(value) => {
            let mut out = base;
            for len in value.len.iter().rev() {
                out = format!(
                    "[{}]{}",
                    render_const_expr(&len.0, packages).unwrap_or_else(|_| "0".to_string()),
                    out
                );
            }
            out
        }
    }
}

pub(crate) fn render_const_expr(
    expr: &hir::ConstExpr,
    packages: &GoPackages,
) -> IdlcResult<String> {
    Ok(crate::generate::render_const_expr(
        expr,
        &|value| go_scoped_name(value, packages),
        &go_literal,
        &|operator| match operator {
            hir::UnaryOperator::Add => "+",
            hir::UnaryOperator::Sub => "-",
            hir::UnaryOperator::Not => "^",
        },
    ))
}

/// Go name of a scoped reference inside the file that renders it.
pub(crate) fn go_scoped_name(value: &hir::ScopedName, packages: &GoPackages) -> String {
    if let Some(reference) = packages.reference(value) {
        return reference;
    }
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

pub(crate) fn go_const_type(ty: &hir::ConstType, packages: &GoPackages) -> String {
    match ty {
        hir::ConstType::IntegerType(value) => go_integer_type(value),
        hir::ConstType::FloatingPtType | hir::ConstType::FixedPtConstType => "float64".to_string(),
        hir::ConstType::CharType | hir::ConstType::WideCharType => "rune".to_string(),
        hir::ConstType::BooleanType => "bool".to_string(),
        hir::ConstType::OctetType => "byte".to_string(),
        hir::ConstType::StringType(_) | hir::ConstType::WideStringType(_) => "string".to_string(),
        hir::ConstType::ScopedName(value) => go_scoped_name(value, packages),
        hir::ConstType::SequenceType(_) => "any".to_string(),
    }
}

pub(crate) fn go_type(ty: &hir::TypeSpec, packages: &GoPackages) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => go_integer_type(value),
        hir::TypeSpec::FloatingPtType => "float64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "rune".to_string(),
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "any".to_string()
        }
        hir::TypeSpec::ScopedName(value) => go_scoped_name(value, packages),
        hir::TypeSpec::SequenceType(seq) => format!("[]{}", go_type(&seq.ty, packages)),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "string".to_string(),
        hir::TypeSpec::FixedPtType(_) => "float64".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!(
                "map[{}]{}",
                go_type(&map.key, packages),
                go_type(&map.value, packages)
            )
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}[{}]",
            go_capitalize(&value.ident),
            value
                .args
                .iter()
                .map(|arg| go_type(arg, packages))
                .collect::<Vec<_>>()
                .join(", "),
        ),
    }
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
