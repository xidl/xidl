use crate::generate::render_const_expr;
use itertools::Itertools;
use serde_json::json;
use std::collections::HashSet;
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
        hir::ConstType::FloatingPtType => "f64".to_string(),
        hir::ConstType::FixedPtConstType => "f64".to_string(),
        hir::ConstType::CharType => "char".to_string(),
        hir::ConstType::WideCharType => "char".to_string(),
        hir::ConstType::BooleanType => "bool".to_string(),
        hir::ConstType::OctetType => "u8".to_string(),
        hir::ConstType::StringType(_) => "&'static str".to_string(),
        hir::ConstType::WideStringType(_) => "&'static str".to_string(),
        hir::ConstType::ScopedName(value) => rust_scoped_name(value),
        hir::ConstType::SequenceType(_) => "*const c_void".to_string(),
    }
}

pub fn rust_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => rust_integer_type(value),
            hir::SimpleTypeSpec::FloatingPtType => "f64".to_string(),
            hir::SimpleTypeSpec::CharType => "char".to_string(),
            hir::SimpleTypeSpec::WideCharType => "char".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::AnyType => "*mut c_void".to_string(),
            hir::SimpleTypeSpec::ObjectType => "*mut c_void".to_string(),
            hir::SimpleTypeSpec::ValueBaseType => "*mut c_void".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => rust_scoped_name(value),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                format!("Vec<{}>", rust_type(&seq.ty))
            }
            hir::TemplateTypeSpec::StringType(_) => "String".to_string(),
            hir::TemplateTypeSpec::WideStringType(_) => "String".to_string(),
            hir::TemplateTypeSpec::FixedPtType(_) => "f64".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                format!(
                    "BTreeMap<{}, {}>",
                    rust_type(&map.key),
                    rust_type(&map.value)
                )
            }
        },
    }
}

pub fn rust_switch_type(value: &hir::SwitchTypeSpec) -> String {
    match value {
        hir::SwitchTypeSpec::IntegerType(ty) => rust_integer_type(ty),
        hir::SwitchTypeSpec::CharType => "char".to_string(),
        hir::SwitchTypeSpec::WideCharType => "char".to_string(),
        hir::SwitchTypeSpec::BooleanType => "bool".to_string(),
        hir::SwitchTypeSpec::ScopedName(value) => rust_scoped_name(value),
        hir::SwitchTypeSpec::OctetType => "u8".to_string(),
    }
}

pub fn rust_literal(value: &hir::Literal) -> String {
    match value {
        hir::Literal::IntegerLiteral(lit) => match lit {
            hir::IntegerLiteral::BinNumber(value) => value.clone(),
            hir::IntegerLiteral::OctNumber(value) => value.clone(),
            hir::IntegerLiteral::DecNumber(value) => value.clone(),
            hir::IntegerLiteral::HexNumber(value) => value.clone(),
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

pub fn array_type(base: &str, dims: &[String]) -> String {
    let mut out = base.to_string();
    for dim in dims.iter().rev() {
        out = format!("[{}; {}]", out, dim);
    }
    out
}

pub fn declarator_dims(decl: &hir::Declarator) -> Vec<String> {
    match decl {
        hir::Declarator::SimpleDeclarator(_) => Vec::new(),
        hir::Declarator::ArrayDeclarator(value) => {
            value.len.iter().map(|len| render_const(&len.0)).collect()
        }
    }
}

pub fn declarator_name(decl: &hir::Declarator) -> String {
    match decl {
        hir::Declarator::SimpleDeclarator(value) => value.0.clone(),
        hir::Declarator::ArrayDeclarator(value) => value.ident.clone(),
    }
}

pub fn type_with_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> String {
    let base = rust_type(ty);
    let dims = declarator_dims(decl);
    if dims.is_empty() {
        base
    } else {
        array_type(&base, &dims)
    }
}

pub fn member_json(ty: &hir::TypeSpec, decl: &hir::Declarator) -> serde_json::Value {
    let name = rust_ident(&declarator_name(decl));
    let ty = type_with_decl(ty, decl);
    json!({ "ty": ty, "name": name })
}

pub fn typedef_json(base: &str, decl: &hir::Declarator) -> serde_json::Value {
    let name = rust_ident(&declarator_name(decl));
    let dims = declarator_dims(decl);
    let ty = if dims.is_empty() {
        base.to_string()
    } else {
        array_type(base, &dims)
    };
    json!({ "ty": ty, "name": name })
}

fn annotation_name_is_derive(annotation: &hir::Annotation) -> bool {
    match annotation {
        hir::Annotation::Builtin { name, .. } => name.eq_ignore_ascii_case("derive"),
        hir::Annotation::ScopedName { name, .. } => name
            .name
            .last()
            .map(|value| value.eq_ignore_ascii_case("derive"))
            .unwrap_or(false),
        _ => false,
    }
}

fn push_derive(out: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
    let item = value.trim();
    if item.is_empty() {
        return;
    }
    if seen.insert(item.to_string()) {
        out.push(item.to_string());
    }
}

pub fn rust_derives_from_annotations(annotations: &[hir::Annotation]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for annotation in annotations {
        if !annotation_name_is_derive(annotation) {
            continue;
        };
        let params = match annotation {
            hir::Annotation::Builtin { params, .. } => params.as_ref(),
            hir::Annotation::ScopedName { params, .. } => params.as_ref(),
            _ => None,
        };
        let Some(params) = params else {
            continue;
        };
        match params {
            hir::AnnotationParams::Raw(value) => {
                for item in value.split(',') {
                    push_derive(&mut out, &mut seen, item);
                }
            }
            hir::AnnotationParams::ConstExpr(expr) => {
                let rendered = render_const_expr(expr, &rust_scoped_name, &rust_literal);
                for item in rendered.split(',') {
                    push_derive(&mut out, &mut seen, item);
                }
            }
            hir::AnnotationParams::Params(values) => {
                for value in values {
                    push_derive(&mut out, &mut seen, &value.ident);
                }
            }
        }
    }
    out
}

pub fn rust_derives_from_annotations_with_extra(
    primary: &[hir::Annotation],
    extra: &[hir::Annotation],
) -> Vec<String> {
    let mut out = rust_derives_from_annotations(primary);
    if extra.is_empty() {
        return out;
    }
    let mut seen: HashSet<String> = out.iter().cloned().collect();
    for derive in rust_derives_from_annotations(extra) {
        if seen.insert(derive.clone()) {
            out.push(derive);
        }
    }
    out
}
