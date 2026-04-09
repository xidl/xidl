#[path = "actions.rs"]
mod actions;
#[path = "formatter.rs"]
mod formatter;
#[path = "helpers.rs"]
mod helpers;
#[path = "indent.rs"]
mod indent;
#[path = "jinja.rs"]
mod jinja;
#[cfg(test)]
mod tests;

use crate::{diagnostic::DiagnosticRunner, error::IdlcResult};
use std::collections::HashMap;

use self::helpers::{build_gap, collect_tokens, ensure_trailing_newline, normalize_blank_lines};

const IDL_QUERY: &str = include_str!("queries/idl.scm");
#[cfg(feature = "fmt-rust")]
const RUST_QUERY: &str = include_str!("queries/rust.scm");
#[cfg(feature = "fmt-cpp")]
const CPP_QUERY: &str = include_str!("queries/cpp.scm");
#[cfg(feature = "fmt-typescript")]
const TYPESCRIPT_QUERY: &str = include_str!("queries/typescript.scm");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InsertKind {
    AppendSpace,
    PrependSpace,
    AppendNewline,
    PrependNewline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Token {
    pub(super) start: usize,
    pub(super) end: usize,
}

#[derive(Clone, Debug)]
struct FormatConfig<'a> {
    language: tree_sitter::Language,
    query_source: &'a str,
    label: &'a str,
    preserve_inline_ws: bool,
    indent_parens: bool,
    normalize_indent: bool,
}

#[derive(Debug, Default)]
pub(super) struct QueryActions {
    pub(super) append: HashMap<usize, Vec<InsertKind>>,
    pub(super) prepend: HashMap<usize, Vec<InsertKind>>,
    pub(super) indent_pre: HashMap<usize, i32>,
    pub(super) indent_post: HashMap<usize, i32>,
    pub(super) comments: HashMap<usize, usize>,
}

pub(super) struct Formatter<'a> {
    config: FormatConfig<'a>,
}

pub fn format_idl_source(source: &str) -> IdlcResult<String> {
    format_idl_source_with_name(source, "input.idl")
}

pub fn format_idl_source_with_name(source: &str, filename: &str) -> IdlcResult<String> {
    DiagnosticRunner::new_idl().run(source, filename)?;
    Formatter::new(FormatConfig {
        language: tree_sitter_idl::language(),
        query_source: IDL_QUERY,
        label: "idl",
        preserve_inline_ws: false,
        indent_parens: true,
        normalize_indent: true,
    })
    .format(source)
    .map(|output| ensure_trailing_newline(&output))
}

#[cfg(feature = "fmt-rust")]
pub fn format_rust_source(source: &str) -> IdlcResult<String> {
    format_tree_sitter_source(source, RUST_QUERY, "rust", false, false, false)
}

#[cfg(not(feature = "fmt-rust"))]
pub fn format_rust_source(source: &str) -> IdlcResult<String> {
    Ok(source.to_string())
}

#[cfg(feature = "fmt-cpp")]
pub fn format_c_source(source: &str) -> IdlcResult<String> {
    format_tree_sitter_source(source, CPP_QUERY, "c", false, false, true)
}

#[cfg(not(feature = "fmt-cpp"))]
pub fn format_c_source(source: &str) -> IdlcResult<String> {
    Ok(source.to_string())
}

#[cfg(feature = "fmt-typescript")]
pub fn format_typescript_source(source: &str) -> IdlcResult<String> {
    format_tree_sitter_source(source, TYPESCRIPT_QUERY, "typescript", false, false, true)
}

#[cfg(not(feature = "fmt-typescript"))]
pub fn format_typescript_source(source: &str) -> IdlcResult<String> {
    Ok(source.to_string())
}

pub fn format_jinja_source(source: &str) -> IdlcResult<String> {
    Ok(normalize_blank_lines(&jinja::normalize_jinja_indentation(
        source,
    )))
}

#[cfg(any(feature = "fmt-rust", feature = "fmt-cpp", feature = "fmt-typescript"))]
fn format_tree_sitter_source(
    source: &str,
    query_source: &str,
    label: &str,
    preserve_inline_ws: bool,
    indent_parens: bool,
    normalize_indent: bool,
) -> IdlcResult<String> {
    let language = match label {
        #[cfg(feature = "fmt-rust")]
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        #[cfg(feature = "fmt-cpp")]
        "c" => tree_sitter_cpp::LANGUAGE.into(),
        #[cfg(feature = "fmt-typescript")]
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        _ => unreachable!(),
    };
    let output = Formatter::new(FormatConfig {
        language,
        query_source,
        label,
        preserve_inline_ws,
        indent_parens,
        normalize_indent,
    })
    .format(source)?;
    Ok(normalize_blank_lines(&output))
}

fn push_action(actions: &mut HashMap<usize, Vec<InsertKind>>, pos: usize, kind: InsertKind) {
    actions.entry(pos).or_default().push(kind);
}

fn mark_comment(actions: &mut QueryActions, start: usize, end: usize) {
    actions.comments.insert(start, end);
    push_action(&mut actions.prepend, start, InsertKind::PrependNewline);
    push_action(&mut actions.append, end, InsertKind::AppendNewline);
}

fn add_indent(map: &mut HashMap<usize, i32>, pos: usize, delta: i32) {
    *map.entry(pos).or_insert(0) += delta;
}

fn append_comment(output: &mut String, source: &str, start: usize, end: usize, indent_level: i32) {
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    for _ in 0..indent_level {
        output.push_str(helpers::INDENT);
    }
    output.push_str(source[start..end].trim());
    output.push('\n');
}
