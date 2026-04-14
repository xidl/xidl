use xidl_parser::hir;

pub(super) fn parse_string_array(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    let unquoted = trim_quotes(trimmed).unwrap_or_else(|| trimmed.to_string());
    split_top_level(&unquoted, ',')
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

pub(super) fn parse_raw_params(raw: &str) -> Vec<(String, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if !trimmed.contains('=') {
        return vec![(
            "value".to_string(),
            trim_quotes(trimmed).unwrap_or_else(|| trimmed.to_string()),
        )];
    }
    split_top_level(trimmed, ',')
        .into_iter()
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            Some((
                key.to_string(),
                trim_quotes(value.trim()).unwrap_or_else(|| value.trim().to_string()),
            ))
        })
        .collect()
}

pub(super) fn trim_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(value[1..value.len() - 1].to_string());
        }
    }
    None
}

pub(super) fn render_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(
        expr,
        &crate::generate::rust::util::rust_scoped_name,
        &crate::generate::rust::util::rust_literal,
    )
}

fn split_top_level(raw: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut escaped = false;
    for ch in raw.chars() {
        if let Some(q) = quote {
            current.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch == separator && bracket_depth == 0 && paren_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}
