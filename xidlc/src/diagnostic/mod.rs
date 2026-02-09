mod colors;
mod theme;

use crate::error::{DiagnosticError, IdlcError, IdlcResult};
use colors::CaptureColors;
use miette::NamedSource;
use miette::highlighters::{
    Highlighter as MietteHighlighter, HighlighterState as MietteHighlighterState,
};
use owo_colors::Style;
use std::collections::BTreeMap;
use std::ops::Range;
use theme::Base16Theme;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

const IDL_HIGHLIGHT_QUERY: &str = tree_sitter_idl::HIGHLIGHTS_QUERY;
const ANSI_RESET: &str = "\x1b[0m";

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

#[derive(Debug, Clone, Copy, Default)]
pub struct IdlMietteHighlighter;

struct IdlMietteHighlighterState {
    colors: CaptureColors,
}

pub struct DiagnosticRunner {
    language: Language,
    label: &'static str,
}

impl DiagnosticRunner {
    pub fn idl() -> Self {
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
        if root.has_error() {
            let error_range = find_error_range(root).unwrap_or(0..0);
            let err = DiagnosticError {
                src: NamedSource::new(name, source.to_owned()).with_language("idl"),
                bad_bit: (error_range.start, error_range.len()).into(),
            };
            return Err(IdlcError::diagnostic(err));
        }

        Ok(())
    }
}

pub fn run_idl_source(source: &str, name: &str) -> IdlcResult<()> {
    DiagnosticRunner::idl().run(source, name)
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
        run_idl_source(source, name)?;

        let tree = self.parse_tree(source)?;
        let root = tree.root_node();

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

        let palette = Base16Theme::dracula();
        let query = Query::new(&tree_sitter_idl::language(), IDL_HIGHLIGHT_QUERY);
        let colors = if let Ok(query) = query {
            let names: Vec<&'static str> = query
                .capture_names()
                .iter()
                .map(|name| {
                    let boxed: Box<str> = name.to_string().into_boxed_str();
                    let leaked: &'static mut str = Box::leak(boxed);
                    &*leaked
                })
                .collect();
            CaptureColors::new(&palette, &names)
        } else {
            CaptureColors::new(&palette, &[])
        };

        Box::new(IdlMietteHighlighterState { colors })
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
        let mut styled: Vec<owo_colors::Styled<&'s str>> = Vec::new();
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
            let _ = self.colors.default_color();
            return vec![Style::default().style(line)];
        }

        spans.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
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

fn find_error_range(root: tree_sitter::Node<'_>) -> Option<Range<usize>> {
    let mut stack = vec![root];
    let mut best: Option<Range<usize>> = None;

    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let range = node.start_byte()..node.end_byte();
            if best
                .as_ref()
                .is_none_or(|current| range.start < current.start)
            {
                best = Some(range);
            }
        }

        if node.child_count() > 0 {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }
    }

    best
}
