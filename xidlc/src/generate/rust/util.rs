use crate::generate::render_const_expr;
use serde_json::json;
use std::collections::{HashMap, HashSet};
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
            hir::SimpleTypeSpec::AnyType => "::serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ObjectType => "::serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ValueBaseType => "::serde_json::Value".to_string(),
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
            hir::TemplateTypeSpec::TemplateType(value) => format!(
                "{}<{}>",
                rust_ident(&value.ident),
                value
                    .args
                    .iter()
                    .map(rust_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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

pub fn constr_type_scoped_name(constr: &hir::ConstrTypeDcl) -> hir::ScopedName {
    let ident = match constr {
        hir::ConstrTypeDcl::StructForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::StructDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::EnumDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionDef(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitsetDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitmaskDcl(def) => def.ident.clone(),
    };
    hir::ScopedName {
        name: vec![ident],
        is_root: false,
    }
}

pub fn serde_rename_from_annotations(annotations: &[hir::Annotation]) -> Option<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("name") {
            continue;
        }
        let value = annotation_params(annotation)
            .map(normalize_annotation_params)
            .and_then(|params| {
                params
                    .get("value")
                    .cloned()
                    .or_else(|| params.get("name").cloned())
            });
        if value.is_some() {
            return value;
        }
    }
    None
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

fn rust_passthrough_attr_name(annotation: &hir::Annotation) -> Option<String> {
    let name = annotation_name(annotation)?;
    let lower = name.to_ascii_lowercase();
    let prefix = "rust-";
    if !lower.starts_with(prefix) {
        return None;
    }
    let attr = &name[prefix.len()..];
    if attr.is_empty() {
        None
    } else {
        Some(attr.to_string())
    }
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

fn normalize_annotation_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match params {
        hir::AnnotationParams::Raw(value) => {
            for (key, value) in parse_raw_annotation_params(value) {
                out.insert(key.to_ascii_lowercase(), value);
            }
        }
        hir::AnnotationParams::Params(values) => {
            for value in values {
                let raw = value
                    .value
                    .as_ref()
                    .map(render_annotation_const_expr)
                    .unwrap_or_default();
                out.insert(
                    value.ident.to_ascii_lowercase(),
                    trim_annotation_quotes(&raw).unwrap_or(raw),
                );
            }
        }
        hir::AnnotationParams::ConstExpr(expr) => {
            let rendered = render_annotation_const_expr(expr);
            out.insert(
                "value".to_string(),
                trim_annotation_quotes(&rendered).unwrap_or(rendered),
            );
        }
    }
    out
}

fn render_annotation_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(expr, &rust_scoped_name, &rust_literal)
}

fn render_rust_passthrough_params(params: &hir::AnnotationParams) -> String {
    match params {
        hir::AnnotationParams::Raw(value) => value.trim().to_string(),
        hir::AnnotationParams::ConstExpr(expr) => {
            render_annotation_const_expr(expr).trim().to_string()
        }
        hir::AnnotationParams::Params(values) => values
            .iter()
            .map(|value| {
                if let Some(expr) = &value.value {
                    format!(
                        "{} = {}",
                        value.ident,
                        render_annotation_const_expr(expr).trim()
                    )
                } else {
                    value.ident.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn trim_annotation_quotes(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        let first = bytes[0] as char;
        let last = bytes[raw.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(raw[1..raw.len() - 1].to_string());
        }
    }
    None
}

fn parse_raw_annotation_params(raw: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut quote = None;
    for ch in raw.chars() {
        match ch {
            '"' | '\'' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                buf.push(ch);
            }
            ',' if quote.is_none() => {
                push_raw_annotation_param(&mut parts, &buf);
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }
    push_raw_annotation_param(&mut parts, &buf);
    parts
}

fn push_raw_annotation_param(parts: &mut Vec<(String, String)>, raw: &str) {
    let raw = raw.trim();
    if raw.is_empty() {
        return;
    }
    if let Some((key, value)) = raw.split_once('=') {
        let key = key.trim();
        let value = value.trim();
        if !key.is_empty() {
            let value = trim_annotation_quotes(value).unwrap_or_else(|| value.to_string());
            parts.push((key.to_string(), value));
        }
    } else {
        let value = trim_annotation_quotes(raw).unwrap_or_else(|| raw.to_string());
        parts.push(("value".to_string(), value));
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

pub fn rust_passthrough_attrs_from_annotations(annotations: &[hir::Annotation]) -> Vec<String> {
    let mut out = Vec::new();
    for annotation in annotations {
        if let Some(attr_name) = rust_passthrough_attr_name(annotation) {
            let rendered = annotation_params(annotation)
                .map(render_rust_passthrough_params)
                .unwrap_or_default();
            if rendered.is_empty() {
                out.push(attr_name);
            } else {
                out.push(format!("{attr_name}({rendered})"));
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

    for d in out.iter_mut() {
        if d == "Serialize" {
            *d = "::serde::Serialize".to_string();
        }
        if d == "Deserialize" {
            *d = "::serde::Deserialize".to_string();
        }
    }
    out
}

pub struct RustDeriveInfo {
    pub all: Vec<String>,
    pub non_serde: Vec<String>,
    pub has_serde_serialize: bool,
    pub has_serde_deserialize: bool,
}

impl RustDeriveInfo {
    pub fn enable_serde_attrs(&self) -> bool {
        self.has_serde_serialize || self.has_serde_deserialize
    }
}

pub fn rust_derive_info_with_extra(
    primary: &[hir::Annotation],
    extra: &[hir::Annotation],
) -> RustDeriveInfo {
    let all = rust_derives_from_annotations_with_extra(primary, extra);
    let has_serde_serialize = all.iter().any(|value| value == "::serde::Serialize");
    let has_serde_deserialize = all.iter().any(|value| value == "::serde::Deserialize");
    let non_serde = all
        .iter()
        .filter(|value| {
            value.as_str() != "::serde::Serialize" && value.as_str() != "::serde::Deserialize"
        })
        .cloned()
        .collect();
    RustDeriveInfo {
        all,
        non_serde,
        has_serde_serialize,
        has_serde_deserialize,
    }
}
