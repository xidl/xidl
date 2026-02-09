use crate::error::DiagnosticError;
use miette::NamedSource;
use std::ops::Range;

pub fn build_parse_diagnostic(
    source: &str,
    name: &str,
    root: tree_sitter::Node<'_>,
) -> DiagnosticError {
    let error_range = find_error_range(root);
    let error_range = error_range.unwrap();

    DiagnosticError {
        src: NamedSource::new(name, source.to_string()),
        bad_bit: (error_range).into(),
    }
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
