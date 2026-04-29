use crate::error::IdlcResult;
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::transport::{TransportDirection, TransportTracker, TypeRegistry};
use convert_case::{Case, Casing};
use xidl_parser::hir;

pub fn rust_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "i8".to_string(),
        hir::IntegerType::UChar => "u8".to_string(),
        hir::IntegerType::U8 => "u8".to_string(),
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I8 => "i8".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}

pub fn axum_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => rust_integer_type(value),
        hir::TypeSpec::FloatingPtType => "f64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "char".to_string(),
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "xidl_rust_axum::serde_json::Value".to_string()
        }
        hir::TypeSpec::ScopedName(value) => render_scoped_name(value),
        hir::TypeSpec::SequenceType(seq) => format!("Vec<{}>", axum_type(&seq.ty)),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "String".to_string(),
        hir::TypeSpec::FixedPtType(_) => "f64".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!(
                "::std::collections::BTreeMap<{}, {}>",
                axum_type(&map.key),
                axum_type(&map.value)
            )
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            rust_ident(&value.ident),
            value
                .args
                .iter()
                .map(axum_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub fn render_scoped_name(value: &hir::ScopedName) -> String {
    let mut iter = value.name.iter();
    let mut parts = Vec::new();
    if let Some(first) = iter.next() {
        if !value.is_root && first == "crate" {
            parts.push("crate".to_string());
        } else {
            parts.push(rust_ident(first));
        }
    }
    for part in iter {
        parts.push(rust_ident(part));
    }
    let path = parts.join("::");
    if value.is_root {
        format!("::{path}")
    } else {
        path
    }
}

pub fn render_param_type(
    ty: &hir::TypeSpec,
    _attr: Option<&hir::ParamAttribute>,
    optional: bool,
) -> String {
    let inner = axum_type(ty);
    if optional {
        format!("Option<{inner}>")
    } else {
        inner
    }
}

pub fn transport_param_type(
    ty: &hir::TypeSpec,
    optional: bool,
    direction: TransportDirection,
    transport: &mut TransportTracker,
    registry: &TypeRegistry,
) -> IdlcResult<String> {
    let inner = transport.map_type(ty, direction, registry)?;
    Ok(if optional {
        format!("Option<{inner}>")
    } else {
        inner
    })
}

pub fn method_struct_prefix(interface_name: &str, method_name: &str) -> String {
    let interface = interface_name.strip_prefix("r#").unwrap_or(interface_name);
    let method = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}",
        interface.to_case(Case::Pascal),
        method.to_case(Case::Pascal)
    )
}
