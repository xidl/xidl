use convert_case::{Case, Casing};
use xidl_parser::hir;

use super::method::TypeRefTarget;

pub(crate) fn ts_scoped_name(
    value: &hir::ScopedName,
    _module_path: &[String],
    target: TypeRefTarget,
) -> String {
    let parts = value
        .name
        .iter()
        .map(|part| ts_ident(part))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return "unknown".to_string();
    }
    let name = parts.join(".");
    match target {
        TypeRefTarget::Types => name,
        TypeRefTarget::Client => format!("types.{name}"),
    }
}

pub(crate) fn zod_schema_ref(
    value: &hir::ScopedName,
    _module_path: &[String],
    prefix: Option<&str>,
) -> String {
    let mut parts = value
        .name
        .iter()
        .map(|part| ts_ident(part))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return "unknownSchema".to_string(); // Fallback
    }
    if let Some(last) = parts.last_mut() {
        *last = format!("{}Schema", last);
    }
    let joined = parts.join(".");
    if let Some(p) = prefix {
        format!("{}.{}", p, joined)
    } else {
        joined
    }
}

pub(crate) fn integer_schema_primitive(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::U64
        | hir::IntegerType::U32
        | hir::IntegerType::U16
        | hir::IntegerType::U8
        | hir::IntegerType::UChar
        | hir::IntegerType::Octet => "coerce.number().int().nonnegative()".to_string(),
        _ => "coerce.number().int()".to_string(),
    }
}

pub(crate) fn ts_ident(value: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        let valid = if idx == 0 {
            ch.is_ascii_alphabetic() || ch == '_'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_'
        };
        if valid {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || is_ts_keyword(&out) {
        format!("_{out}")
    } else {
        out
    }
}

pub(crate) fn ts_prop_name(value: &str) -> String {
    if is_valid_ts_ident(value) && !is_ts_keyword(value) {
        value.to_string()
    } else {
        format!("\"{}\"", value)
    }
}

fn is_valid_ts_ident(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_ts_keyword(value: &str) -> bool {
    matches!(
        value,
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "as"
            | "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
            | "any"
            | "boolean"
            | "constructor"
            | "declare"
            | "get"
            | "module"
            | "require"
            | "number"
            | "set"
            | "string"
            | "symbol"
            | "type"
            | "from"
            | "of"
    )
}

pub(crate) fn declarator_name(decl: &hir::Declarator) -> &str {
    match decl {
        hir::Declarator::SimpleDeclarator(simple) => &simple.0,
        hir::Declarator::ArrayDeclarator(array) => &array.ident,
    }
}

pub(crate) fn method_struct_prefix(interface_name: &str, method_name: &str) -> String {
    let interface = interface_name.strip_prefix("r#").unwrap_or(interface_name);
    let method = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}",
        interface.to_case(Case::Pascal),
        method.to_case(Case::Pascal)
    )
}

pub(crate) fn scoped_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ts_ident(ident)
    } else {
        let mut parts = module_path
            .iter()
            .map(|part| ts_ident(part))
            .collect::<Vec<_>>();
        parts.push(ts_ident(ident));
        parts.join(".")
    }
}

pub(crate) fn default_path(
    module_path: &[String],
    interface_name: &str,
    method_name: &str,
) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    format!("/{}", parts.join("/"))
}

pub(crate) fn indent_block(value: &str, level: usize) -> String {
    let indent = "    ".repeat(level);
    value
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
