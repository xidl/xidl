//! Syntax diagnostic pass over tree-sitter parse trees.

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::error::{ParseError, ParserResult};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator, Tree};

const IDL_ERROR_QUERY: &str = "[(ERROR) (MISSING)] @error";

/// Collects syntax diagnostics from an IDL tree-sitter parse tree.
pub struct SyntaxPass;

impl SyntaxPass {
    /// Parses `source` once with IDL normalization and returns syntax diagnostics.
    pub fn check_source(source: &str) -> ParserResult<Vec<Diagnostic>> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_idl::language())
            .map_err(|err| ParseError::TreeSitterError(format!("xidl parser: {err}")))?;
        let normalized = crate::parser::normalize_source_for_tree_sitter(source);
        let tree = parser.parse(normalized.as_ref(), None).ok_or_else(|| {
            ParseError::TreeSitterError("xidl parser: failed to parse IDL source".to_string())
        })?;
        Ok(Self::collect_tree(&tree, source))
    }

    /// Scans an already-parsed tree-sitter `Tree` for `ERROR` and `MISSING` nodes.
    pub fn collect_tree(tree: &Tree, source: &str) -> Vec<Diagnostic> {
        let root = tree.root_node();
        if !root.has_error() {
            return Vec::new();
        }
        let Ok(query) = Query::new(&tree_sitter_idl::language(), IDL_ERROR_QUERY) else {
            return Vec::new();
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        let mut diagnostics = Vec::new();

        while let Some(matched) = matches.next() {
            for capture in matched.captures {
                diagnostics.push(Self::node_to_diagnostic(capture.node, source));
            }
        }

        diagnostics.sort_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end));
        diagnostics.dedup_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end));
        diagnostics
    }

    fn node_to_diagnostic(node: Node<'_>, source: &str) -> Diagnostic {
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

        Diagnostic::error(
            DiagnosticCode::SyntaxError,
            offset..offset.saturating_add(len),
            message,
        )
    }
}
