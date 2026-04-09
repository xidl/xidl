mod doc_comments;
mod normalize;

use crate::error::ParserResult;
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

pub struct ParseContext<'a> {
    pub source: &'a [u8],
    pub symbols: HashMap<String, String>,
    doc_consumed: HashSet<usize>,
}

impl<'a> ParseContext<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            symbols: HashMap::new(),
            doc_consumed: HashSet::new(),
        }
    }

    pub fn node_text(&self, node: &Node) -> ParserResult<&str> {
        Ok(node.utf8_text(self.source)?)
    }

    pub fn take_doc_comment(&mut self, node: &Node) -> Option<String> {
        let start = node.start_byte();
        if self.doc_consumed.contains(&start) {
            return None;
        }
        let doc = doc_comments::extract(self.source, start);
        if doc.is_some() {
            self.doc_consumed.insert(start);
        }
        doc
    }
}

pub trait FromTreeSitter<'a>: Sized {
    fn from_node(node: Node<'a>, context: &mut ParseContext<'a>) -> ParserResult<Self>;
}

impl<'a> FromTreeSitter<'a> for String {
    fn from_node(node: Node<'a>, context: &mut ParseContext<'a>) -> ParserResult<Self> {
        Ok(context.node_text(&node)?.to_string())
    }
}

impl<'a, T> FromTreeSitter<'a> for Box<T>
where
    T: FromTreeSitter<'a>,
{
    fn from_node(node: Node<'a>, context: &mut ParseContext<'a>) -> ParserResult<Self> {
        Ok(Box::new(T::from_node(node, context)?))
    }
}

pub fn parser_text(text: &str) -> ParserResult<crate::typed_ast::Specification> {
    use crate::typed_ast::Specification;

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_idl::language()).unwrap();

    let normalized = normalize::source(text);

    let tree = parser.parse(normalized.as_ref(), None).ok_or_else(|| {
        crate::error::ParseError::TreeSitterError("Failed to parse text".to_string())
    })?;

    let root_node = tree.root_node();
    if root_node.has_error() {
        return Err(crate::error::ParseError::TreeSitterError(
            "Failed to parse text".to_string(),
        ));
    }
    let mut context = ParseContext::new(text.as_bytes());

    Specification::from_node(root_node, &mut context)
}

pub fn normalize_source_for_tree_sitter(text: &str) -> std::borrow::Cow<'_, str> {
    normalize::source(text)
}

#[cfg(test)]
mod tests;
