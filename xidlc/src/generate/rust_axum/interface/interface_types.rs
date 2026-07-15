use crate::generate::rust::util::rust_ident;
use xidl_parser::hir;

pub(crate) fn render_param_type(ty: &hir::TypeSpec, optional: bool) -> String {
    let inner = axum_type(ty);
    if optional {
        format!("Option<{inner}>")
    } else {
        inner
    }
}

pub(crate) fn axum_type(ty: &hir::TypeSpec) -> String {
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
        hir::TypeSpec::MapType(map) => format!(
            "::std::collections::BTreeMap<{}, {}>",
            axum_type(&map.key),
            axum_type(&map.value)
        ),
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

fn render_scoped_name(value: &hir::ScopedName) -> String {
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

fn rust_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "i8".to_string(),
        hir::IntegerType::UChar | hir::IntegerType::Octet | hir::IntegerType::U8 => {
            "u8".to_string()
        }
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I8 => "i8".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}

pub(crate) fn header_is_multi(ty: &hir::TypeSpec) -> bool {
    matches!(ty, hir::TypeSpec::SequenceType(_))
}

pub(crate) fn header_item_ty(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SequenceType(seq) => axum_type(&seq.ty),
        _ => axum_type(ty),
    }
}

pub(crate) fn header_item_is_string(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::SequenceType(seq) => header_item_is_string(&seq.ty),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => true,
        _ => false,
    }
}

pub(crate) fn header_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::SequenceType(seq) => header_item_is_primitive(&seq.ty),
        hir::TypeSpec::IntegerType(_) | hir::TypeSpec::FloatingPtType | hir::TypeSpec::Boolean => {
            true
        }
        _ => false,
    }
}

pub(crate) fn cookie_is_multi(ty: &hir::TypeSpec) -> bool {
    header_is_multi(ty)
}

pub(crate) fn cookie_item_ty(ty: &hir::TypeSpec) -> String {
    header_item_ty(ty)
}

pub(crate) fn cookie_item_is_string(ty: &hir::TypeSpec) -> bool {
    header_item_is_string(ty)
}

pub(crate) fn cookie_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    header_item_is_primitive(ty)
}
