mod colors;
mod theme;

use crate::error::{DiagnosticError, IdlcError, IdlcResult};
use miette::LabeledSpan;
use miette::highlighters::{
    Highlighter as MietteHighlighter, HighlighterState as MietteHighlighterState,
};
use owo_colors::Style;
use std::collections::BTreeMap;
use std::ops::Range;
use theme::Base16Theme;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

const IDL_HIGHLIGHT_QUERY: &str = tree_sitter_idl::HIGHLIGHTS_QUERY;
const IDL_ERROR_QUERY: &str = "[(ERROR) (MISSING)] @error";

#[derive(Clone, Debug)]
pub struct HighlightNode {
    pub range: Range<usize>,
    pub capture: String,
}

#[derive(Clone, Debug)]
pub struct HighlightedText {
    pub text: String,
    #[allow(dead_code)]
    pub nodes: BTreeMap<usize, HighlightNode>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IdlMietteHighlighter;

struct IdlMietteHighlighterState;

pub struct DiagnosticRunner {
    language: Language,
    label: &'static str,
}

impl DiagnosticRunner {
    pub fn new_idl() -> Self {
        Self {
            language: tree_sitter_idl::language(),
            label: "idl",
        }
    }

    pub fn run(&self, source: &str, name: &str) -> IdlcResult<()> {
        let tree = self.parse(source)?;
        self.ensure_tree(tree, source, name)
    }

    fn parse(&self, source: &str) -> IdlcResult<Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.language)
            .map_err(|err| IdlcError::fmt(format!("set {} language: {err}", self.label)))?;

        parser
            .parse(source, None)
            .ok_or_else(|| IdlcError::fmt(format!("failed to parse {}", self.label)))
    }

    fn ensure_tree(&self, tree: Tree, source: &str, name: &str) -> IdlcResult<()> {
        let root = tree.root_node();
        let mut labels = self.collect_error_labels(root, source);

        if !labels.is_empty() {
            labels.sort_by_key(|label| (label.offset(), label.len()));
            labels.dedup_by_key(|label| (label.offset(), label.len()));

            let diagnostics = labels
                .into_iter()
                .map(|label| DiagnosticError::from_label(name, source, label))
                .collect();
            return Err(IdlcError::diagnostics(diagnostics));
        }

        Ok(())
    }

    fn collect_error_labels(&self, root: tree_sitter::Node<'_>, source: &str) -> Vec<LabeledSpan> {
        let Ok(query) = Query::new(&self.language, IDL_ERROR_QUERY) else {
            return vec![];
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        let mut labels = vec![];

        while let Some(matched) = matches.next() {
            for capture in matched.captures {
                let node = capture.node;
                let start = node.start_byte();
                let mut len = node.end_byte().saturating_sub(start);
                let mut offset = start;
                if len == 0 {
                    if source.is_empty() {
                        offset = 0;
                        len = 0;
                    } else if start >= source.len() {
                        offset = source.len().saturating_sub(1);
                        len = 1;
                    } else {
                        len = 1;
                    }
                }

                let message = if node.is_missing() {
                    let missing = node
                        .utf8_text(source.as_bytes())
                        .ok()
                        .filter(|text| !text.trim().is_empty())
                        .unwrap_or(node.kind());
                    format!("missing {missing}")
                } else {
                    "syntax error".to_string()
                };
                labels.push(LabeledSpan::at(offset..offset + len, message));
            }
        }

        labels
    }
}

pub fn run_idl_source(source: &str, name: &str) -> IdlcResult<()> {
    DiagnosticRunner::new_idl().run(source, name)
}

impl MietteHighlighter for IdlMietteHighlighter {
    fn start_highlighter_state<'h>(
        &'h self,
        source: &dyn miette::SpanContents<'_>,
    ) -> Box<dyn MietteHighlighterState + 'h> {
        let language_is_idl = source
            .language()
            .map(|lang| lang.eq_ignore_ascii_case("idl"))
            .unwrap_or(false);

        if !language_is_idl {
            return Box::new(miette::highlighters::BlankHighlighterState);
        }

        Box::new(IdlMietteHighlighterState)
    }
}

impl MietteHighlighterState for IdlMietteHighlighterState {
    fn highlight_line<'s>(&mut self, line: &'s str) -> Vec<owo_colors::Styled<&'s str>> {
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_idl::language()).is_err() {
            return vec![Style::default().style(line)];
        }

        let Some(tree) = parser.parse(line, None) else {
            return vec![Style::default().style(line)];
        };

        let language = tree_sitter_idl::language();
        let Ok(query) = Query::new(&language, IDL_HIGHLIGHT_QUERY) else {
            return vec![Style::default().style(line)];
        };

        let root = tree.root_node();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, line.as_bytes());
        let mut spans: Vec<(usize, usize, usize)> = Vec::new();

        while let Some(matched) = matches.next() {
            for capture in matched.captures {
                let node = capture.node;
                let start = node.start_byte();
                let end = node.end_byte();
                if end <= start || start > line.len() || end > line.len() {
                    continue;
                }
                spans.push((start, end, capture.index as usize));
            }
        }

        if spans.is_empty() {
            return vec![Style::default().style(line)];
        }

        spans.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));

        let mut styled: Vec<owo_colors::Styled<&'s str>> = Vec::new();
        let mut pos = 0usize;
        for (mut start, end, capture_index) in spans {
            if end <= pos {
                continue;
            }
            if start < pos {
                start = pos;
            }

            if pos < start {
                styled.push(Style::default().style(&line[pos..start]));
            }

            let capture_name = query.capture_names()[capture_index];
            let rgb = colors::color_for_capture(&Base16Theme::dracula(), capture_name);
            let style = Style::new().truecolor(rgb.r, rgb.g, rgb.b);
            styled.push(style.style(&line[start..end]));
            pos = end;
        }

        if pos < line.len() {
            styled.push(Style::default().style(&line[pos..]));
        }

        styled
    }
}
