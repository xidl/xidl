use crate::error::ParserResult;
use std::collections::HashMap;
use tree_sitter::Node;

pub struct ParseContext<'a> {
    pub source: &'a [u8],
    pub symbols: HashMap<String, String>,
}

impl<'a> ParseContext<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            symbols: HashMap::new(),
        }
    }

    pub fn node_text(&self, node: &Node) -> ParserResult<&str> {
        Ok(node.utf8_text(self.source)?)
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

pub fn parser_text(text: &str) -> ParserResult<crate::ast::Specification> {
    use crate::ast::Specification;

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_idl::language()).unwrap();

    let tree = parser.parse(text, None).ok_or_else(|| {
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
