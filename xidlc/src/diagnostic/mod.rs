mod colors;
mod theme;

mod highlight;
pub use highlight::TreeSitterMietteHighlighter;

use crate::error::{DiagnosticError, IdlcError, IdlcResult};
use miette::LabeledSpan;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

const IDL_ERROR_QUERY: &str = "[(ERROR) (MISSING)] @error";

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

    pub fn new_cpp() -> Self {
        Self {
            language: tree_sitter_cpp::LANGUAGE.into(),
            label: "cpp",
        }
    }

    pub fn run(&self, source: &str, filename: &str) -> IdlcResult<()> {
        let tree = self.parse(source)?;
        self.ensure_tree(tree, source, filename)
    }

    fn parse(&self, source: &str) -> IdlcResult<Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.language)
            .map_err(|err| IdlcError::fmt(format!("set {} language: {err}", self.label)))?;
        let normalized = if self.label == "idl" {
            xidl_parser::parser::normalize_source_for_tree_sitter(source)
        } else {
            std::borrow::Cow::Borrowed(source)
        };

        parser
            .parse(normalized.as_ref(), None)
            .ok_or_else(|| IdlcError::fmt(format!("failed to parse {}", self.label)))
    }

    fn ensure_tree(&self, tree: Tree, source: &str, filename: &str) -> IdlcResult<()> {
        let root = tree.root_node();
        let mut labels = self.collect_error_labels(root, source);

        if !labels.is_empty() {
            labels.sort_by_key(|label| (label.offset(), label.len()));
            labels.dedup_by_key(|label| (label.offset(), label.len()));

            let diagnostics = labels
                .into_iter()
                .map(|label| DiagnosticError::from_label(filename, source, label))
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
