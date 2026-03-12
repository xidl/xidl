use xidl_parser::hir;

pub fn doc_lines_from_annotations(annotations: &[hir::Annotation]) -> Vec<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("doc") {
            continue;
        }
        let params = match annotation {
            hir::Annotation::Builtin { params, .. } => params.as_ref(),
            hir::Annotation::ScopedName { params, .. } => params.as_ref(),
            _ => None,
        };
        if let Some(params) = params {
            if let Some(text) = doc_text_from_params(params) {
                return text.lines().map(|line| line.to_string()).collect();
            }
        }
        return Vec::new();
    }
    Vec::new()
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn doc_text_from_params(params: &hir::AnnotationParams) -> Option<String> {
    match params {
        hir::AnnotationParams::Raw(value) => parse_raw_doc(value),
        hir::AnnotationParams::Params(values) => values.iter().find_map(|value| {
            if !value.ident.eq_ignore_ascii_case("value")
                && !value.ident.eq_ignore_ascii_case("text")
                && !value.ident.eq_ignore_ascii_case("doc")
            {
                return None;
            }
            value.value.as_ref().and_then(const_expr_string)
        }),
        hir::AnnotationParams::ConstExpr(expr) => const_expr_string(expr),
    }
}

fn parse_raw_doc(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0] as char;
        let last = bytes[bytes.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(unescape_doc(&raw[1..raw.len() - 1]));
        }
    }
    None
}

fn const_expr_string(expr: &hir::ConstExpr) -> Option<String> {
    primary_string(&expr.0)
}

fn primary_string(expr: &hir::OrExpr) -> Option<String> {
    match expr {
        hir::OrExpr::XorExpr(value) => xor_string(value),
        hir::OrExpr::OrExpr(_, _) => None,
    }
}

fn xor_string(expr: &hir::XorExpr) -> Option<String> {
    match expr {
        hir::XorExpr::AndExpr(value) => and_string(value),
        hir::XorExpr::XorExpr(_, _) => None,
    }
}

fn and_string(expr: &hir::AndExpr) -> Option<String> {
    match expr {
        hir::AndExpr::ShiftExpr(value) => shift_string(value),
        hir::AndExpr::AndExpr(_, _) => None,
    }
}

fn shift_string(expr: &hir::ShiftExpr) -> Option<String> {
    match expr {
        hir::ShiftExpr::AddExpr(value) => add_string(value),
        hir::ShiftExpr::LeftShiftExpr(_, _) | hir::ShiftExpr::RightShiftExpr(_, _) => None,
    }
}

fn add_string(expr: &hir::AddExpr) -> Option<String> {
    match expr {
        hir::AddExpr::MultExpr(value) => mult_string(value),
        hir::AddExpr::AddExpr(_, _) | hir::AddExpr::SubExpr(_, _) => None,
    }
}

fn mult_string(expr: &hir::MultExpr) -> Option<String> {
    match expr {
        hir::MultExpr::UnaryExpr(value) => unary_string(value),
        hir::MultExpr::MultExpr(_, _)
        | hir::MultExpr::DivExpr(_, _)
        | hir::MultExpr::ModExpr(_, _) => None,
    }
}

fn unary_string(expr: &hir::UnaryExpr) -> Option<String> {
    match expr {
        hir::UnaryExpr::PrimaryExpr(value) => primary_expr_string(value),
        hir::UnaryExpr::UnaryExpr(_, _) => None,
    }
}

fn primary_expr_string(expr: &hir::PrimaryExpr) -> Option<String> {
    match expr {
        hir::PrimaryExpr::Literal(value) => literal_string(value),
        hir::PrimaryExpr::ScopedName(_) | hir::PrimaryExpr::ConstExpr(_) => None,
    }
}

fn literal_string(lit: &hir::Literal) -> Option<String> {
    match lit {
        hir::Literal::StringLiteral(value) => Some(unescape_doc(value)),
        hir::Literal::WideStringLiteral(value) => Some(unescape_doc(value)),
        _ => None,
    }
}

fn unescape_doc(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}
