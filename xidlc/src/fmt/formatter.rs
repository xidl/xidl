use crate::error::{IdlcError, IdlcResult};
use std::collections::HashMap;
use tree_sitter::{Parser, Query};

use super::helpers::needs_token_separator;
use super::{
    FormatConfig, Formatter, InsertKind, QueryActions, Token, append_comment, build_gap,
    collect_tokens,
};

impl<'a> Formatter<'a> {
    pub(super) fn new(config: FormatConfig<'a>) -> Self {
        Self { config }
    }

    pub(super) fn format(&self, source: &str) -> IdlcResult<String> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.config.language)
            .map_err(|err| IdlcError::fmt(format!("set {} language: {err}", self.config.label)))?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| IdlcError::fmt(format!("failed to parse {}", self.config.label)))?;
        let root = tree.root_node();
        if root.has_error() {
            if cfg!(test) {
                unreachable!()
            }
            return Err(IdlcError::fmt(format!("parse {} error", self.config.label)));
        }
        let query = Query::new(&self.config.language, self.config.query_source)
            .map_err(|err| IdlcError::fmt(format!("query error: {err}")))?;
        let actions = super::actions::collect_actions(source, root, &query);
        let tokens = collect_tokens(root);
        self.rebuild_source(source, &tokens, &actions)
    }

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
            indent_level = apply_indent(indent_level, actions.indent_post.get(&prev_end));
            indent_level = apply_indent(indent_level, actions.indent_pre.get(&token.start));
            let token_text = &source[token.start..token.end];
            let comment_end = actions.comments.get(&token.start).copied();
            let is_comment_token = comment_end.is_some()
                || token_text.starts_with("//")
                || token_text.starts_with("/*");
            if let Some(comment_end) = comment_end {
                append_comment(&mut output, source, token.start, comment_end, indent_level);
                prev_token = Some(Token {
                    start: token.start,
                    end: comment_end,
                });
                prev_was_comment = true;
                prev_end = comment_end;
                continue;
            }
            self.append_gap(
                &mut output,
                source,
                gap,
                prev_end,
                token,
                prev_token,
                prev_was_comment,
                indent_level,
                actions,
            );
            output.push_str(token_text);
            prev_token = Some(*token);
            prev_was_comment = is_comment_token;
            prev_end = token.end;
        }
        self.append_tail(&mut output, source, prev_end, indent_level, actions);
        if self.config.normalize_indent {
            Ok(super::indent::normalize_indentation(
                &output,
                self.config.indent_parens,
            ))
        } else {
            Ok(output)
        }
    }

    fn append_gap(
        &self,
        output: &mut String,
        source: &str,
        gap: &str,
        prev_end: usize,
        token: &Token,
        prev_token: Option<Token>,
        prev_was_comment: bool,
        indent_level: i32,
        actions: &QueryActions,
    ) {
        if prev_was_comment && gap.chars().all(|c| c.is_whitespace()) {
            return;
        }
        if !gap.chars().all(|c| c.is_whitespace()) {
            output.push_str(gap);
            return;
        }
        let append = actions_for(&actions.append, prev_end);
        let prepend = actions_for(&actions.prepend, token.start);
        if append.is_empty() && prepend.is_empty() && self.config.preserve_inline_ws {
            output.push_str(gap);
        } else if append.is_empty() && prepend.is_empty() && gap.contains('\n') {
            output.push_str(&"\n".repeat(gap.chars().filter(|c| *c == '\n').count()));
            for _ in 0..indent_level {
                output.push_str(super::helpers::INDENT);
            }
        } else if append.is_empty() && prepend.is_empty() {
            if let Some(prev) = prev_token {
                if needs_token_separator(
                    &source[prev.start..prev.end],
                    &source[token.start..token.end],
                ) {
                    output.push(' ');
                }
            }
        } else {
            let empty_block = prev_token
                .map(|prev| &source[prev.start..prev.end] == "{")
                .unwrap_or(false)
                && &source[token.start..token.end] == "}";
            output.push_str(&build_gap(append, prepend, indent_level, empty_block));
        }
    }

    fn append_tail(
        &self,
        output: &mut String,
        source: &str,
        prev_end: usize,
        indent_level: i32,
        actions: &QueryActions,
    ) {
        let tail = &source[prev_end..];
        let indent_level = apply_indent(indent_level, actions.indent_post.get(&prev_end));
        if tail.chars().all(|c| c.is_whitespace()) {
            output.push_str(&build_gap(
                actions_for(&actions.append, prev_end),
                Vec::new(),
                indent_level,
                false,
            ));
        } else {
            output.push_str(tail);
        }
    }
}

fn apply_indent(current: i32, delta: Option<&i32>) -> i32 {
    (current + delta.copied().unwrap_or(0)).max(0)
}

fn actions_for(actions: &HashMap<usize, Vec<InsertKind>>, pos: usize) -> Vec<InsertKind> {
    actions.get(&pos).cloned().unwrap_or_default()
}
