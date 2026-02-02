use crate::highlight::HighlightNode;
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceSpan};
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

#[derive(Debug)]
struct ParseDiagnostic {
    source: NamedSource<String>,
    name: String,
    span: Option<SourceSpan>,
    label: Option<String>,
}

impl fmt::Display for ParseDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to parse idl: {}", self.name)
    }
}

impl std::error::Error for ParseDiagnostic {}

impl Diagnostic for ParseDiagnostic {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        self.span.map(|span| {
            let label = self.label.clone();
            let iter = std::iter::once(LabeledSpan::new_with_span(label, span));
            Box::new(iter) as Box<dyn Iterator<Item = LabeledSpan>>
        })
    }
}

pub fn build_parse_diagnostic(
    source: &str,
    name: &str,
    nodes: &BTreeMap<usize, HighlightNode>,
    root: tree_sitter::Node<'_>,
) -> miette::Report {
    let error_range = find_error_range(root);
    let mut label = None;
    let span = error_range.map(|range| {
        let start = range.start;
        let len = range.end.saturating_sub(range.start).max(1);
        if let Some((_, node)) = nodes.range(..=start).next_back()
            && start >= node.range.start
            && start <= node.range.end
        {
            label = Some(format!("parse error near {}", node.capture));
        }
        SourceSpan::new(start.into(), len)
    });
    miette::Report::new(ParseDiagnostic {
        source: NamedSource::new(name, source.to_string()),
        name: name.to_string(),
        span,
        label,
    })
}

pub fn find_error_range(root: tree_sitter::Node<'_>) -> Option<Range<usize>> {
    let mut stack = vec![root];
    let mut best: Option<Range<usize>> = None;
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let range = node.start_byte()..node.end_byte();
            if best.as_ref().is_none_or(|b| range.start < b.start) {
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
