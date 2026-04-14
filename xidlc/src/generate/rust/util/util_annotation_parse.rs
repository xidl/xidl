use xidl_parser::hir;

pub(crate) fn trim_annotation_quotes(raw: &str) -> Option<String> {
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

pub(crate) fn render_annotation_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(
        expr,
        &crate::generate::rust::util::rust_scoped_name,
        &crate::generate::rust::util::rust_literal,
    )
}

pub(crate) fn parse_raw_annotation_params(raw: &str) -> Vec<(String, String)> {
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
