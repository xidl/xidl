use crate::generate::rust::util::rust_ident;
use itertools::Itertools;
use xidl_parser::hir;

pub(super) fn jsonrpc_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => rust_integer_type(value),
        hir::TypeSpec::FloatingPtType => "f64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "char".to_string(),
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "serde_json::Value".to_string()
        }
        hir::TypeSpec::ScopedName(value) => render_scoped_name(value),
        hir::TypeSpec::SequenceType(seq) => format!("Vec<{}>", jsonrpc_type(&seq.ty)),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "String".to_string(),
        hir::TypeSpec::FixedPtType(_) => "f64".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!(
                "BTreeMap<{}, {}>",
                jsonrpc_type(&map.key),
                jsonrpc_type(&map.value)
            )
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            rust_ident(&value.ident),
            value
                .args
                .iter()
                .map(jsonrpc_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
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
        hir::IntegerType::UChar | hir::IntegerType::Octet | hir::IntegerType::U8 => {
            "u8".to_string()
        }
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}
