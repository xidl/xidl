use super::helpers::INDENT;

pub(super) fn normalize_jinja_indentation(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut control_depth: i32 = 0;
    let mut content_brace_depth: i32 = 0;
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }
        let is_control = trimmed.starts_with("{%") && trimmed.ends_with("%}");
        let control_stmt = extract_jinja_control_statement(trimmed);
        if let Some(statement) = control_stmt {
            if is_jinja_control_end(statement) || is_jinja_control_mid(statement) {
                control_depth = (control_depth - 1).max(0);
            }
        }
        let line_content_depth = if is_control {
            content_brace_depth
        } else {
            let leading_closing = trimmed.chars().take_while(|&ch| ch == '}').count() as i32;
            (content_brace_depth - leading_closing).max(0)
        };
        for _ in 0..(control_depth + line_content_depth).max(0) {
            out.push_str(INDENT);
        }
        out.push_str(trimmed);
        out.push('\n');
        if is_control {
            if let Some(statement) = control_stmt {
                if is_jinja_control_start(statement) || is_jinja_control_mid(statement) {
                    control_depth += 1;
                }
            }
        } else {
            content_brace_depth = next_content_brace_depth(trimmed, content_brace_depth);
        }
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

fn extract_jinja_control_statement(line: &str) -> Option<&str> {
    if !(line.starts_with("{%") && line.ends_with("%}")) {
        return None;
    }
    let mut body = line
        .trim_start_matches("{%-")
        .trim_start_matches("{%+")
        .trim_start_matches("{%")
        .trim_end_matches("-%}")
        .trim_end_matches("+%}")
        .trim_end_matches("%}")
        .trim();
    if body.is_empty() {
        return None;
    }
    if let Some(rest) = body.strip_prefix('#') {
        body = rest.trim();
    }
    body.split_whitespace().next()
}

fn is_jinja_control_start(statement: &str) -> bool {
    matches!(
        statement,
        "for"
            | "if"
            | "with"
            | "call"
            | "macro"
            | "filter"
            | "block"
            | "trans"
            | "autoescape"
            | "set"
    )
}

fn is_jinja_control_mid(statement: &str) -> bool {
    matches!(statement, "elif" | "else" | "pluralize")
}

fn is_jinja_control_end(statement: &str) -> bool {
    matches!(
        statement,
        "endfor"
            | "endif"
            | "endblock"
            | "endwith"
            | "endfilter"
            | "endmacro"
            | "endcall"
            | "endset"
            | "endtrans"
            | "endautoescape"
    )
}

fn next_content_brace_depth(line: &str, current: i32) -> i32 {
    let mut depth = current;
    let mut in_string = false;
    let mut in_char = false;
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if in_string {
            if ch == '\\' && index + 1 < bytes.len() {
                index += 2;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if in_char {
            if ch == '\\' && index + 1 < bytes.len() {
                index += 2;
                continue;
            }
            if ch == '\'' {
                in_char = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '\'' {
            in_char = true;
        } else if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = (depth - 1).max(0);
        }
        index += 1;
    }
    depth
}
