use std::borrow::Cow;

pub fn source(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut index = 0usize;
    let mut quote = None;

    while index < bytes.len() {
        let ch = bytes[index] as char;

        if let Some(current_quote) = quote {
            out.push(ch);
            if ch == '\\' && index + 1 < bytes.len() {
                index += 1;
                out.push(bytes[index] as char);
            } else if ch == current_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if matches!(ch, '"' | '\'') {
            quote = Some(ch);
            out.push(ch);
            index += 1;
            continue;
        }

        if ch == '@' {
            let (next_index, annotation_changed) = normalize_annotation(bytes, index, &mut out);
            changed |= annotation_changed;
            index = next_index;
            continue;
        }

        out.push(ch);
        index += 1;
    }

    if changed {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(text)
    }
}

fn normalize_annotation(bytes: &[u8], start: usize, out: &mut String) -> (usize, bool) {
    out.push('@');
    let mut index = start + 1;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':') {
            out.push(ch);
            index += 1;
        } else {
            break;
        }
    }

    if index < bytes.len() && bytes[index] as char == '(' {
        if let Some(end) = masked_annotation_end(bytes, index) {
            out.push('(');
            for _ in index + 1..end {
                out.push(' ');
            }
            out.push(')');
            return (end + 1, true);
        }
    }

    (index, false)
}

fn masked_annotation_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start + 1;
    let mut quote = None;
    let mut depth = 1usize;
    let mut has_bracket = false;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        if let Some(current_quote) = quote {
            if ch == '\\' && index + 1 < bytes.len() {
                index += 2;
                continue;
            }
            if ch == current_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '[' | ']' => has_bracket = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return has_bracket.then_some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }

    None
}
