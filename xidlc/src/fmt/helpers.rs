use super::{InsertKind, Token};

pub(super) const INDENT: &str = "    ";

pub(super) fn collect_tokens(root: tree_sitter::Node<'_>) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.child_count() == 0 {
            let start = node.start_byte();
            let end = node.end_byte();
            if start != end {
                tokens.push(Token { start, end });
            }
            continue;
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    tokens.sort_by_key(|token| token.start);
    tokens
}

pub(super) fn build_gap(
    append: Vec<InsertKind>,
    prepend: Vec<InsertKind>,
    indent_level: i32,
    empty_block: bool,
) -> String {
    let mut newline_count = 0usize;
    let mut has_space = false;
    for action in append.iter().chain(prepend.iter()) {
        match action {
            InsertKind::AppendNewline | InsertKind::PrependNewline => newline_count += 1,
            InsertKind::AppendSpace | InsertKind::PrependSpace => has_space = true,
        }
    }
    if newline_count > 0 {
        if empty_block && newline_count > 1 {
            newline_count = 1;
        }
        let mut out = "\n".repeat(newline_count);
        for _ in 0..indent_level {
            out.push_str(INDENT);
        }
        out
    } else if has_space {
        " ".to_string()
    } else {
        String::new()
    }
}

pub(super) fn needs_token_separator(prev: &str, next: &str) -> bool {
    let Some(prev_char) = prev.chars().last() else {
        return false;
    };
    let Some(next_char) = next.chars().next() else {
        return false;
    };
    let prev_is_word = prev_char.is_ascii_alphanumeric() || prev_char == '_';
    let next_is_word = next_char.is_ascii_alphanumeric() || next_char == '_';
    (prev_is_word && next_is_word) || (prev_char == ':' && next_char == ':')
}

pub(super) fn normalize_blank_lines(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut started = false;
    let mut blank_run = 0usize;
    let mut pending_blank = false;
    for line in input.lines() {
        let trimmed = line.trim_end_matches('\r');
        if trimmed.trim().is_empty() {
            if !started {
                continue;
            }
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
            pending_blank = true;
            continue;
        }
        if pending_blank && trimmed.trim() != "}" {
            out.push('\n');
        }
        pending_blank = false;
        started = true;
        blank_run = 0;
        out.push_str(trimmed);
        out.push('\n');
    }
    while out.ends_with('\n') {
        out.pop();
    }
    ensure_trailing_newline(&out)
}

pub(super) fn ensure_trailing_newline(input: &str) -> String {
    format!("{}\n", input.trim_end())
}
