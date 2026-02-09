use crate::error::{DiagnosticError, IdlcError, IdlcResult};
use miette::NamedSource;
use std::ops::Range;
use tree_sitter::{Language, Parser, Tree};

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
                src: NamedSource::new(name, source.to_owned()),
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
