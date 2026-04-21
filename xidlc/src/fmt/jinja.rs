use super::helpers::INDENT;

pub(super) fn normalize_jinja_indentation(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut control_depth: i32 = 0;
    let mut content_brace_depth: i32 = 0;
    for line in input.lines() {
        let line = line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }

        let content = strip_leading_indent(line);
        let control_tag = parse_jinja_control_tag(content.trim());
        if let Some(tag) = control_tag {
            if is_jinja_control_end(tag.statement) || is_jinja_control_mid(tag.statement) {
                control_depth = (control_depth - 1).max(0);
            }
        }

        let line_content_depth = if control_tag.is_some() {
            content_brace_depth
        } else {
            let leading_closing = content
                .trim_start()
                .chars()
                .take_while(|&ch| ch == '}')
                .count() as i32;
            (content_brace_depth - leading_closing).max(0)
        };

        for _ in 0..(control_depth + line_content_depth).max(0) {
            out.push_str(INDENT);
        }
        out.push_str(content);
        out.push('\n');

        if let Some(tag) = control_tag {
            if is_jinja_control_start(tag) || is_jinja_control_mid(tag.statement) {
                control_depth += 1;
            }
        } else {
            content_brace_depth = next_content_brace_depth(content, content_brace_depth);
        }
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

#[derive(Clone, Copy)]
struct JinjaControlTag<'a> {
    statement: &'a str,
    body: &'a str,
}

pub(super) fn should_skip_jinja_format(input: &str) -> bool {
    let mut in_block_comment = false;
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_block_comment {
                continue;
            }
            continue;
        }

        if in_block_comment {
            if trimmed.contains("jinja-fmt: skip") {
                return true;
            }
            if trimmed.contains("#}") {
                in_block_comment = false;
            }
            continue;
        }

        if trimmed.starts_with("##") {
            if trimmed.contains("jinja-fmt: skip") {
                return true;
            }
            continue;
        }

        if trimmed.starts_with("{#") {
            if trimmed.contains("jinja-fmt: skip") {
                return true;
            }
            if !trimmed.contains("#}") {
                in_block_comment = true;
            }
            continue;
        }

        return false;
    }

    false
}

fn parse_jinja_control_tag(line: &str) -> Option<JinjaControlTag<'_>> {
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
    let statement = body.split_whitespace().next()?;
    Some(JinjaControlTag { statement, body })
}

fn is_jinja_control_start(tag: JinjaControlTag<'_>) -> bool {
    matches!(
        tag.statement,
        "for" | "if" | "with" | "call" | "macro" | "filter" | "block" | "trans" | "autoescape"
    ) || (tag.statement == "set" && !tag.body.contains('='))
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

fn strip_leading_indent(line: &str) -> &str {
    line.trim_start_matches([' ', '\t'])
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
