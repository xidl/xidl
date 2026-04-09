use super::helpers::INDENT;

pub(super) fn normalize_indentation(input: &str, indent_parens: bool) -> String {
    let mut out = String::with_capacity(input.len());
    let mut brace_depth: i32 = 0;
    let mut paren_depth: i32 = 0;
    let mut case_depth: Option<i32> = None;
    let mut in_block_comment = false;
    for line in input.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            out.push('\n');
            continue;
        }
        let trimmed_line = line.trim_start();
        let is_case_label =
            trimmed_line.starts_with("case ") || trimmed_line.starts_with("default");
        let mut line_brace_depth = brace_depth;
        let mut line_paren_depth = paren_depth;
        if trimmed_line.starts_with('}') {
            line_brace_depth = (line_brace_depth - 1).max(0);
        }
        if trimmed_line.starts_with(')') {
            line_paren_depth = (line_paren_depth - 1).max(0);
        }
        let mut indent = line_brace_depth + if indent_parens { line_paren_depth } else { 0 };
        if !is_case_label {
            if let Some(depth) = case_depth {
                if line_brace_depth >= depth {
                    indent += 1;
                }
            }
        }
        for _ in 0..indent {
            out.push_str(INDENT);
        }
        out.push_str(trimmed_line);
        out.push('\n');
        update_depths(
            line,
            &mut brace_depth,
            &mut paren_depth,
            &mut in_block_comment,
        );
        if is_case_label {
            case_depth = Some(line_brace_depth);
        } else if let Some(depth) = case_depth {
            if line_brace_depth < depth {
                case_depth = None;
            }
        }
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

fn update_depths(
    line: &str,
    brace_depth: &mut i32,
    paren_depth: &mut i32,
    in_block_comment: &mut bool,
) {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    let mut in_char = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if *in_block_comment {
            if ch == '*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                *in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_string {
            if ch == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if in_char {
            if ch == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if ch == '\'' {
                in_char = false;
            }
            i += 1;
            continue;
        }
        if ch == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            break;
        }
        if ch == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            *in_block_comment = true;
            i += 2;
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '\'' {
            in_char = true;
        } else if ch == '{' {
            *brace_depth += 1;
        } else if ch == '}' {
            *brace_depth = (*brace_depth - 1).max(0);
        } else if ch == '(' {
            *paren_depth += 1;
        } else if ch == ')' {
            *paren_depth = (*paren_depth - 1).max(0);
        }
        i += 1;
    }
}
