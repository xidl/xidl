use crate::generate::rust::util::rust_ident;
use itertools::Itertools;
use xidl_parser::hir;

pub(super) fn attr_return_type(ty: &hir::TypeSpec) -> String {
    jsonrpc_type(ty)
}

pub(super) fn render_param_type(ty: &hir::TypeSpec, attr: Option<&hir::ParamAttribute>) -> String {
    let _ = attr;
    jsonrpc_type(ty)
}

pub(super) fn jsonrpc_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => rust_integer_type(value),
            hir::SimpleTypeSpec::FloatingPtType => "f64".to_string(),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => "char".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => "serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => render_scoped_name(value),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => format!("Vec<{}>", jsonrpc_type(&seq.ty)),
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                "String".to_string()
            }
            hir::TemplateTypeSpec::FixedPtType(_) => "f64".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                format!(
                    "BTreeMap<{}, {}>",
                    jsonrpc_type(&map.key),
                    jsonrpc_type(&map.value)
                )
            }
            hir::TemplateTypeSpec::TemplateType(value) => format!(
                "{}<{}>",
                rust_ident(&value.ident),
                value
                    .args
                    .iter()
                    .map(jsonrpc_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
    }
}

fn render_scoped_name(value: &hir::ScopedName) -> String {
    let path = value.name.iter().map(|part| rust_ident(part)).join("::");
    if value.is_root {
        format!("::{path}")
    } else {
        path
    }
}

fn rust_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char | hir::IntegerType::I8 => "i8".to_string(),
        hir::IntegerType::UChar | hir::IntegerType::U8 => "u8".to_string(),
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}
