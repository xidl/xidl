use xidl_parser::hir;

use super::super::GoScope;
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
    scope: GoScope<'_>,
) -> String {
    let base = go_type(ty, scope);
    match decl {
        hir::Declarator::SimpleDeclarator(_) => base,
        hir::Declarator::ArrayDeclarator(value) => {
            let mut out = base;
            for len in value.len.iter().rev() {
                out = format!("[{}]{}", render_const_expr(&len.0, scope), out);
            }
            out
        }
    }
}

pub(crate) fn render_const_expr(expr: &hir::ConstExpr, scope: GoScope<'_>) -> String {
    crate::generate::render_const_expr(
        expr,
        &|value| scope.reference(value),
        &go_literal,
        &|operator| match operator {
            hir::UnaryOperator::Add => "+",
            hir::UnaryOperator::Sub => "-",
            hir::UnaryOperator::Not => "^",
        },
    )
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

pub(crate) fn go_const_type(ty: &hir::ConstType, scope: GoScope<'_>) -> String {
    match ty {
        hir::ConstType::IntegerType(value) => go_integer_type(value),
        hir::ConstType::FloatingPtType | hir::ConstType::FixedPtConstType => "float64".to_string(),
        hir::ConstType::CharType | hir::ConstType::WideCharType => {
            scope.packages.runtime_type("Char")
        }
        hir::ConstType::BooleanType => "bool".to_string(),
        hir::ConstType::OctetType => "byte".to_string(),
        hir::ConstType::StringType(_) | hir::ConstType::WideStringType(_) => "string".to_string(),
        hir::ConstType::ScopedName(value) => scope.reference(value),
        hir::ConstType::SequenceType(_) => "any".to_string(),
    }
}

pub(crate) fn go_type(ty: &hir::TypeSpec, scope: GoScope<'_>) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => go_integer_type(value),
        hir::TypeSpec::FloatingPtType => "float64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => {
            scope.packages.runtime_type("Char")
        }
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "any".to_string()
        }
        hir::TypeSpec::ScopedName(value) => scope.reference(value),
        hir::TypeSpec::SequenceType(seq) => format!("[]{}", go_type(&seq.ty, scope)),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "string".to_string(),
        hir::TypeSpec::FixedPtType(_) => "float64".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!(
                "map[{}]{}",
                go_type(&map.key, scope),
                go_type(&map.value, scope)
            )
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}[{}]",
            go_capitalize(&value.ident),
            value
                .args
                .iter()
                .map(|arg| go_type(arg, scope))
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
