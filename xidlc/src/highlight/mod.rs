mod colors;
mod diagnostics;
mod theme;

use crate::error::{IdlcError, IdlcResult};
use colors::CaptureColors;
use diagnostics::build_parse_diagnostic;
use std::collections::BTreeMap;
use std::ops::Range;
use theme::Base16Theme;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

const IDL_HIGHLIGHT_QUERY: &str = tree_sitter_idl::HIGHLIGHTS_QUERY;

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

pub struct IdlHighlighter {
    language: tree_sitter::Language,
    config: HighlightConfiguration,
    capture_names: Vec<String>,
    colors: CaptureColors,
}

impl IdlHighlighter {
    pub fn new() -> IdlcResult<Self> {
        let language = tree_sitter_idl::language();
        let highlight_names = collect_capture_names(language.clone())?;
        let mut config =
            HighlightConfiguration::new(language.clone(), "idl", IDL_HIGHLIGHT_QUERY, "", "")
                .map_err(|err| IdlcError::fmt(format!("highlight config error: {err}")))?;
        config.configure(&highlight_names);
        let palette = Base16Theme::dracula();
        let colors = CaptureColors::new(&palette, &highlight_names);
        let capture_names = highlight_names
            .iter()
            .map(|name| name.to_string())
            .collect();
        Ok(Self {
            language,
            config,
            capture_names,
            colors,
        })
    }

    pub fn highlight(&self, source: &str, name: &str) -> IdlcResult<HighlightedText> {
        let tree = self.parse_tree(source)?;
        let root = tree.root_node();
        if root.has_error() {
            let nodes = self.collect_nodes(root, source)?;
            let report = build_parse_diagnostic(source, name, &nodes, root);
            return Err(IdlcError::diagnostic(report));
        }

        let mut highlighter = Highlighter::new();
        let events = highlighter
            .highlight(&self.config, source.as_bytes(), None, |_| None)
            .map_err(|err| IdlcError::fmt(format!("highlight error: {err}")))?;

        let mut styled = String::with_capacity(source.len());
        let mut stack: Vec<&str> = Vec::new();
        let nodes = self.collect_nodes(root, source)?;
        for event in events {
            match event.map_err(|err| IdlcError::fmt(format!("highlight error: {err}")))? {
                HighlightEvent::Source { start, end } => {
                    let span = &source[start..end];
                    self.push_source_span(&mut styled, span, &stack);
                }
                HighlightEvent::HighlightStart(highlight) => {
                    let name = self.capture_name(highlight.0);
                    styled.push_str(self.colors.color_for(name));
                    stack.push(name);
                }
                HighlightEvent::HighlightEnd => {
                    stack.pop();
                    styled.push_str(ANSI_RESET);
                    if let Some(prev) = stack.last() {
                        styled.push_str(self.colors.color_for(prev));
                    }
                }
            }
        }

        Ok(HighlightedText {
            text: styled,
            nodes,
        })
    }

    fn parse_tree(&self, source: &str) -> IdlcResult<tree_sitter::Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.language)
            .map_err(|err| IdlcError::fmt(format!("set idl language: {err}")))?;

        parser
            .parse(source, None)
            .ok_or_else(|| IdlcError::fmt("failed to parse idl".to_string()))
    }

    fn push_source_span(&self, styled: &mut String, span: &str, stack: &[&str]) {
        if stack.is_empty() {
            styled.push_str(self.colors.default_color());
            styled.push_str(span);
            styled.push_str(ANSI_RESET);
        } else {
            styled.push_str(span);
        }
    }

    fn collect_nodes(
        &self,
        root: tree_sitter::Node<'_>,
        source: &str,
    ) -> IdlcResult<BTreeMap<usize, HighlightNode>> {
        let query = Query::new(&self.language, IDL_HIGHLIGHT_QUERY)
            .map_err(|err| IdlcError::fmt(format!("highlight query error: {err}")))?;
        let mut cursor = QueryCursor::new();
        let mut nodes: BTreeMap<usize, HighlightNode> = BTreeMap::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        while let Some(matched) = matches.next() {
            for capture in matched.captures {
                let name = query.capture_names()[capture.index as usize];
                let node = capture.node;
                nodes.insert(
                    node.start_byte(),
                    HighlightNode {
                        range: node.start_byte()..node.end_byte(),
                        capture: name.to_string(),
                    },
                );
            }
        }
        Ok(nodes)
    }

    fn capture_name(&self, index: usize) -> &str {
        self.capture_names
            .get(index)
            .map(|s| s.as_str())
            .unwrap_or("text")
    }
}

const ANSI_RESET: &str = "\x1b[0m";

fn collect_capture_names(language: tree_sitter::Language) -> IdlcResult<Vec<&'static str>> {
    let query = Query::new(&language, IDL_HIGHLIGHT_QUERY)
        .map_err(|err| IdlcError::fmt(format!("highlight query error: {err}")))?;
    let mut names: Vec<&'static str> = Vec::new();
    for name in query.capture_names() {
        let boxed: Box<str> = name.to_string().into_boxed_str();
        let static_name: &'static str = Box::leak(boxed);
        if !names.contains(&static_name) {
            names.push(static_name);
        }
    }
    Ok(names)
}

pub fn highlight_idl(source: &str, name: &str) -> IdlcResult<HighlightedText> {
    IdlHighlighter::new()?.highlight(source, name)
}

#[allow(dead_code)]
pub fn parse_with_miette(source: &str, name: &str) -> IdlcResult<()> {
    let highlighter = IdlHighlighter::new()?;
    highlighter.highlight(source, name).map(|_| ())
}
