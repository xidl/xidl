#[cfg(test)]
mod tests;

use crate::error::{IdlcError, IdlcResult};
use std::collections::HashMap;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

const IDL_QUERY: &str = include_str!("queries/idl.scm");
const RUST_QUERY: &str = include_str!("queries/rust.scm");
const CPP_QUERY: &str = include_str!("queries/cpp.scm");
const TYPESCRIPT_QUERY: &str = include_str!("queries/typescript.scm");
const INDENT: &str = "    ";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InsertKind {
    AppendSpace,
    PrependSpace,
    AppendNewline,
    PrependNewline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Token {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug)]
struct FormatConfig<'a> {
    language: tree_sitter::Language,
    query_source: &'a str,
    label: &'a str,
    allow_parse_error: bool,
    preserve_inline_ws: bool,
    indent_parens: bool,
    normalize_indent: bool,
}

#[derive(Debug, Default)]
struct QueryActions {
    append: HashMap<usize, Vec<InsertKind>>,
    prepend: HashMap<usize, Vec<InsertKind>>,
    indent_pre: HashMap<usize, i32>,
    indent_post: HashMap<usize, i32>,
    comments: HashMap<usize, usize>,
}

struct Formatter<'a> {
    config: FormatConfig<'a>,
}

pub fn format_idl_source(source: &str) -> IdlcResult<String> {
    Formatter::new(FormatConfig {
        language: tree_sitter_idl::language(),
        query_source: IDL_QUERY,
        label: "idl",
        allow_parse_error: false,
        preserve_inline_ws: false,
        indent_parens: true,
        normalize_indent: true,
    })
    .format(source)
}

pub fn format_rust_source(source: &str) -> IdlcResult<String> {
    let output = Formatter::new(FormatConfig {
        language: tree_sitter_rust::LANGUAGE.into(),
        query_source: RUST_QUERY,
        label: "rust",
        allow_parse_error: false,
        preserve_inline_ws: false,
        indent_parens: false,
        normalize_indent: false,
    })
    .format(source)?;
    Ok(Formatter::normalize_blank_lines(&output))
}

pub fn format_c_source(source: &str) -> IdlcResult<String> {
    let output = Formatter::new(FormatConfig {
        language: tree_sitter_cpp::LANGUAGE.into(),
        query_source: CPP_QUERY,
        label: "c",
        allow_parse_error: true,
        preserve_inline_ws: false,
        indent_parens: false,
        normalize_indent: true,
    })
    .format(source)?;
    Ok(Formatter::normalize_blank_lines(&output))
}

pub fn format_typescript_source(source: &str) -> IdlcResult<String> {
    let output = Formatter::new(FormatConfig {
        language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        query_source: TYPESCRIPT_QUERY,
        label: "typescript",
        allow_parse_error: true,
        preserve_inline_ws: false,
        indent_parens: false,
        normalize_indent: true,
    })
    .format(source)?;
    Ok(Formatter::normalize_blank_lines(&output))
}

pub fn format_jinja_source(source: &str) -> IdlcResult<String> {
    Ok(Formatter::normalize_blank_lines(
        &Formatter::normalize_jinja_indentation(source),
    ))
}

impl<'a> Formatter<'a> {
    fn new(config: FormatConfig<'a>) -> Self {
        Self { config }
    }

    fn format(&self, source: &str) -> IdlcResult<String> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.config.language)
            .map_err(|err| IdlcError::fmt(format!("set {} language: {err}", self.config.label)))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| IdlcError::fmt(format!("failed to parse {}", self.config.label)))?;
        let root = tree.root_node();
        if root.has_error() {
            if cfg!(debug_assertions) {
                unreachable!();
            }
            if self.config.allow_parse_error {
                return Ok(source.to_string());
            }
            return Err(IdlcError::fmt(format!("{} parse error", self.config.label)));
        }

        let query = Query::new(&self.config.language, self.config.query_source)
            .map_err(|err| IdlcError::fmt(format!("query error: {err}")))?;

        let actions = self.collect_actions(source, root, &query);
        let tokens = Self::collect_tokens(root);
        self.rebuild_source(source, &tokens, &actions)
    }

    fn collect_actions(
        &self,
        source: &str,
        root: tree_sitter::Node<'_>,
        query: &Query,
    ) -> QueryActions {
        let mut actions = QueryActions::default();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root, source.as_bytes());
        while let Some(matched) = matches.next() {
            for capture in matched.captures {
                let name = &query.capture_names()[capture.index as usize];
                let node = capture.node;
                match *name {
                    "append-space" => {
                        actions
                            .append
                            .entry(node.end_byte())
                            .or_default()
                            .push(InsertKind::AppendSpace);
                    }
                    "prepend-space" => {
                        actions
                            .prepend
                            .entry(node.start_byte())
                            .or_default()
                            .push(InsertKind::PrependSpace);
                    }
                    "append-newline" => {
                        actions
                            .append
                            .entry(node.end_byte())
                            .or_default()
                            .push(InsertKind::AppendNewline);
                    }
                    "prepend-newline" => {
                        actions
                            .prepend
                            .entry(node.start_byte())
                            .or_default()
                            .push(InsertKind::PrependNewline);
                    }
                    "add-ident" => {
                        Self::add_indent(&mut actions.indent_post, node.end_byte(), 1);
                    }
                    "dec-ident" => {
                        Self::add_indent(&mut actions.indent_pre, node.start_byte(), -1);
                    }
                    "comment" => {
                        actions.comments.insert(node.start_byte(), node.end_byte());
                        actions
                            .prepend
                            .entry(node.start_byte())
                            .or_default()
                            .push(InsertKind::PrependNewline);
                        actions
                            .append
                            .entry(node.end_byte())
                            .or_default()
                            .push(InsertKind::AppendNewline);
                    }
                    _ => {}
                }
            }
        }
        actions
    }

    fn add_indent(map: &mut HashMap<usize, i32>, pos: usize, delta: i32) {
        *map.entry(pos).or_insert(0) += delta;
    }

    fn collect_tokens(root: tree_sitter::Node<'_>) -> Vec<Token> {
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
}
impl<'a> Formatter<'a> {
    fn rebuild_source(
        &self,
        source: &str,
        tokens: &[Token],
        actions: &QueryActions,
    ) -> IdlcResult<String> {
        let mut output = String::with_capacity(source.len());
        let mut indent_level: i32 = 0;
        let mut prev_end: usize = 0;
        let mut prev_token: Option<Token> = None;
        let mut prev_was_comment = false;

        for token in tokens {
            if token.end <= prev_end {
                continue;
            }

            let gap = &source[prev_end..token.start];
            indent_level = Self::apply_indent(indent_level, actions.indent_post.get(&prev_end));
            indent_level = Self::apply_indent(indent_level, actions.indent_pre.get(&token.start));

            let token_text = &source[token.start..token.end];
            let comment_end = actions.comments.get(&token.start).copied();
            let is_comment_token = comment_end.is_some()
                || token_text.starts_with("//")
                || token_text.starts_with("/*");

            if let Some(comment_end) = comment_end {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                for _ in 0..indent_level {
                    output.push_str(INDENT);
                }
                output.push_str(source[token.start..comment_end].trim());
                output.push('\n');
                prev_token = Some(Token {
                    start: token.start,
                    end: comment_end,
                });
                prev_was_comment = true;
                prev_end = comment_end;
                continue;
            }

            if prev_was_comment && gap.chars().all(|c| c.is_whitespace()) {
            } else if gap.chars().all(|c| c.is_whitespace()) {
                let append = Self::actions_for(&actions.append, prev_end);
                let prepend = Self::actions_for(&actions.prepend, token.start);
                if append.is_empty() && prepend.is_empty() && self.config.preserve_inline_ws {
                    output.push_str(gap);
                } else if append.is_empty() && prepend.is_empty() && gap.contains('\n') {
                    let count = gap.chars().filter(|c| *c == '\n').count();
                    output.push_str(&"\n".repeat(count));
                    for _ in 0..indent_level {
                        output.push_str(INDENT);
                    }
                } else if append.is_empty() && prepend.is_empty() {
                    if let Some(prev) = prev_token {
                        if Self::needs_token_separator(&source[prev.start..prev.end], token_text) {
                            output.push(' ');
                        }
                    }
                } else {
                    let empty_block = prev_token
                        .map(|prev| &source[prev.start..prev.end] == "{")
                        .unwrap_or(false)
                        && &source[token.start..token.end] == "}";
                    let insert = Self::build_gap(append, prepend, indent_level, empty_block);
                    output.push_str(&insert);
                }
            } else {
                output.push_str(gap);
            }

            output.push_str(token_text);
            prev_token = Some(*token);
            prev_was_comment = is_comment_token;
            prev_end = token.end;
        }

        let tail = &source[prev_end..];
        indent_level = Self::apply_indent(indent_level, actions.indent_post.get(&prev_end));
        if tail.chars().all(|c| c.is_whitespace()) {
            let insert = Self::build_gap(
                Self::actions_for(&actions.append, prev_end),
                Vec::new(),
                indent_level,
                false,
            );
            output.push_str(&insert);
        } else {
            output.push_str(tail);
        }

        if self.config.normalize_indent {
            Ok(Self::normalize_indentation(
                &output,
                self.config.indent_parens,
            ))
        } else {
            Ok(output)
        }
    }

    fn apply_indent(current: i32, delta: Option<&i32>) -> i32 {
        let next = current + delta.copied().unwrap_or(0);
        next.max(0)
    }

    fn build_gap(
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

    fn actions_for(actions: &HashMap<usize, Vec<InsertKind>>, pos: usize) -> Vec<InsertKind> {
        actions.get(&pos).cloned().unwrap_or_default()
    }

    fn needs_token_separator(prev: &str, next: &str) -> bool {
        let Some(prev_char) = prev.chars().last() else {
            return false;
        };
        let Some(next_char) = next.chars().next() else {
            return false;
        };

        let prev_is_word = prev_char.is_ascii_alphanumeric() || prev_char == '_';
        let next_is_word = next_char.is_ascii_alphanumeric() || next_char == '_';

        if prev_is_word && next_is_word {
            return true;
        }

        prev_char == ':' && next_char == ':'
    }

    fn normalize_indentation(input: &str, indent_parens: bool) -> String {
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
            #[allow(clippy::collapsible_if)]
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

            let bytes = line.as_bytes();
            let mut i = 0;
            let mut in_string = false;
            let mut in_char = false;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if in_block_comment {
                    if ch == '*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        in_block_comment = false;
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
                    in_block_comment = true;
                    i += 2;
                    continue;
                }
                if ch == '"' {
                    in_string = true;
                    i += 1;
                    continue;
                }
                if ch == '\'' {
                    in_char = true;
                    i += 1;
                    continue;
                }
                if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth = (brace_depth - 1).max(0);
                } else if ch == '(' {
                    paren_depth += 1;
                } else if ch == ')' {
                    paren_depth = (paren_depth - 1).max(0);
                }
                i += 1;
            }

            #[allow(clippy::collapsible_if)]
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

    fn normalize_blank_lines(input: &str) -> String {
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
        out
    }

    fn normalize_jinja_indentation(input: &str) -> String {
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
            let control_stmt = Self::extract_jinja_control_statement(trimmed);

            if let Some(statement) = control_stmt {
                //
                if Self::is_jinja_control_end(statement) || Self::is_jinja_control_mid(statement) {
                    control_depth = (control_depth - 1).max(0);
                }
            }

            let line_content_depth = if is_control {
                content_brace_depth
            } else {
                let leading_closing = trimmed.chars().take_while(|&ch| ch == '}').count() as i32;
                (content_brace_depth - leading_closing).max(0)
            };
            let indent_level = (control_depth + line_content_depth).max(0);
            for _ in 0..indent_level {
                out.push_str(INDENT);
            }
            out.push_str(trimmed);
            out.push('\n');

            if is_control {
                //
                if let Some(statement) = control_stmt {
                    //
                    if Self::is_jinja_control_start(statement)
                        || Self::is_jinja_control_mid(statement)
                    {
                        control_depth += 1;
                    }
                }
                continue;
            }

            content_brace_depth = Self::next_content_brace_depth(trimmed, content_brace_depth);
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
}
