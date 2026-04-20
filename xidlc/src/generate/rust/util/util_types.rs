use crate::generate::render_const_expr;
use xidl_parser::hir;

pub fn rust_scoped_name(value: &hir::ScopedName) -> String {
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
    let mut name = parts.join("::");
    if value.is_root {
        name = "::".to_string() + &name;
    }
    name
}

pub fn rust_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "i8".to_string(),
        hir::IntegerType::UChar | hir::IntegerType::U8 => "u8".to_string(),
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I8 => "i8".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}

pub fn rust_ident(value: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
        "final", "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
    ];
    if KEYWORDS.contains(&value) {
        format!("r#{value}")
    } else {
        value.to_string()
    }
}

pub fn rust_const_type(ty: &hir::ConstType) -> String {
    match ty {
        hir::ConstType::IntegerType(value) => rust_integer_type(value),
        hir::ConstType::FloatingPtType | hir::ConstType::FixedPtConstType => "f64".to_string(),
        hir::ConstType::CharType | hir::ConstType::WideCharType => "char".to_string(),
        hir::ConstType::BooleanType => "bool".to_string(),
        hir::ConstType::OctetType => "u8".to_string(),
        hir::ConstType::StringType(_) | hir::ConstType::WideStringType(_) => {
            "&'static str".to_string()
        }
        hir::ConstType::ScopedName(value) => rust_scoped_name(value),
        hir::ConstType::SequenceType(_) => "*const c_void".to_string(),
    }
}

pub fn rust_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => rust_integer_type(value),
        hir::TypeSpec::FloatingPtType => "f64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "char".to_string(),
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "::serde_json::Value".to_string()
        }
        hir::TypeSpec::ScopedName(value) => rust_scoped_name(value),
        hir::TypeSpec::SequenceType(seq) => format!("Vec<{}>", rust_type(&seq.ty)),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "String".to_string(),
        hir::TypeSpec::FixedPtType(_) => "f64".to_string(),
        hir::TypeSpec::MapType(map) => format!(
            "::std::collections::BTreeMap<{}, {}>",
            rust_type(&map.key),
            rust_type(&map.value)
        ),
        hir::TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            rust_ident(&value.ident),
            value
                .args
                .iter()
                .map(rust_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub fn rust_switch_type(value: &hir::SwitchTypeSpec) -> String {
    match value {
        hir::SwitchTypeSpec::IntegerType(ty) => rust_integer_type(ty),
        hir::SwitchTypeSpec::CharType | hir::SwitchTypeSpec::WideCharType => "char".to_string(),
        hir::SwitchTypeSpec::BooleanType => "bool".to_string(),
        hir::SwitchTypeSpec::ScopedName(value) => rust_scoped_name(value),
        hir::SwitchTypeSpec::OctetType => "u8".to_string(),
    }
}

pub fn rust_literal(value: &hir::Literal) -> String {
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

pub fn serialize_kind_name(kind: hir::SerializeKind) -> &'static str {
    match kind {
        hir::SerializeKind::Cdr => "Cdr",
        hir::SerializeKind::PlainCdr => "PlainCdr",
        hir::SerializeKind::PlCdr => "PlCdr",
        hir::SerializeKind::PlainCdr2 => "PlainCdr2",
        hir::SerializeKind::DelimitedCdr => "DelimitedCdr",
        hir::SerializeKind::PlCdr2 => "PlCdr2",
    }
}

pub fn render_const(expr: &hir::ConstExpr) -> String {
    render_const_expr(expr, &rust_scoped_name, &rust_literal)
}

pub fn bitfield_type(value: &hir::BitFieldType) -> String {
    match value {
        hir::BitFieldType::Bool => "bool".to_string(),
        hir::BitFieldType::Octec => "u8".to_string(),
        hir::BitFieldType::SignedInt => "i32".to_string(),
        hir::BitFieldType::UnsignedInt => "u32".to_string(),
    }
}
