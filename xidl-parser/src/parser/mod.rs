mod doc_comments;
mod normalize;
pub(crate) mod raw;

use crate::error::ParserResult;
use std::collections::{HashMap, HashSet};
use tree_sitter::{InputEdit, Node, Parser, Point, Tree};

pub struct ParseContext<'a> {
    pub source: &'a [u8],
    pub symbols: HashMap<String, String>,
    doc_consumed: HashSet<usize>,
}

pub trait IncludeResolver {
    fn resolve(&mut self, parent_path: Option<&str>, path: &str) -> ParserResult<(String, String)>;
}

struct NoopIncludeResolver;
impl IncludeResolver for NoopIncludeResolver {
    fn resolve(
        &mut self,
        _parent_path: Option<&str>,
        _path: &str,
    ) -> ParserResult<(String, String)> {
        Err(crate::error::ParseError::Message(
            "Include resolution not supported in this context".to_string(),
        ))
    }
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
    parser_text_with_resolver(text, None, &mut NoopIncludeResolver)
}

pub fn parser_text_with_resolver(
    text: &str,
    initial_path: Option<&str>,
    resolver: &mut dyn IncludeResolver,
) -> ParserResult<crate::typed_ast::Specification> {
    use crate::typed_ast::Specification;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_idl::language())
        .map_err(|err| {
            crate::error::ParseError::TreeSitterError(format!(
                "failed to load the IDL tree-sitter grammar: {err}"
            ))
        })?;

    let mut source = text.to_string();
    let mut tree_source = normalize::source(text).into_owned();

    let mut tree = parser.parse(&tree_source, None).ok_or_else(|| {
        crate::error::ParseError::TreeSitterError("Failed to parse text".to_string())
    })?;

    if let Some(path) = initial_path {
        let source_len = tree_source.len();
        let mut expander = IncludeExpander {
            source: &mut source,
            tree_source: &mut tree_source,
            tree: &mut tree,
            parser: &mut parser,
            resolver,
            include_stack: Vec::new(),
        };
        expander.expand(path.to_string(), 0, source_len)?;
    }

    let root_node = tree.root_node();
    if root_node.has_error() {
        return Err(crate::error::ParseError::TreeSitterError(
            "Failed to parse text".to_string(),
        ));
    }
    let mut context = ParseContext::new(source.as_bytes());

    Specification::from_node(root_node, &mut context)
}

struct IncludeExpander<'a> {
    source: &'a mut String,
    tree_source: &'a mut String,
    tree: &'a mut Tree,
    parser: &'a mut Parser,
    resolver: &'a mut dyn IncludeResolver,
    include_stack: Vec<String>,
}

impl IncludeExpander<'_> {
    fn expand(
        &mut self,
        current_path: String,
        start_offset: usize,
        mut end_offset: usize,
    ) -> ParserResult<usize> {
        if self.include_stack.contains(&current_path) {
            let chain = self.include_stack.join(" -> ");
            return Err(crate::error::ParseError::Message(format!(
                "cyclic include detected: {} -> {}",
                chain, current_path
            )));
        }
        self.include_stack.push(current_path.clone());

        let mut search_start = start_offset;
        while let Some(node) =
            find_first_include_in_range(self.tree.root_node(), search_start, end_offset)
        {
            let node_start = node.start_byte();
            let node_end = node.end_byte();
            let node_start_pos = node.start_position();
            let node_end_pos = node.end_position();

            let path_str = extract_include_path(node, self.tree_source)?;

            // A versioned include is spliced while the HIR is lowered: the version
            // projection only exists there, so the directive stays in the tree.
            if crate::hir::split_version_selector(&path_str).1.is_some() {
                search_start = node_end;
                continue;
            }

            let (actual_path, content) = self.resolver.resolve(Some(&current_path), &path_str)?;

            let normalized_content = normalize::source(&content);
            let new_len = normalized_content.len();

            // Edit tree
            let edit = InputEdit {
                start_byte: node_start,
                old_end_byte: node_end,
                new_end_byte: node_start + new_len,
                start_position: node_start_pos,
                old_end_position: node_end_pos,
                new_end_position: get_pos_after(node_start_pos, &normalized_content),
            };
            self.tree.edit(&edit);

            // Update source and tree_source
            self.source.replace_range(node_start..node_end, &content);
            self.tree_source
                .replace_range(node_start..node_end, &normalized_content);

            // Re-parse
            *self.tree = self
                .parser
                .parse(&*self.tree_source, Some(self.tree))
                .ok_or_else(|| {
                    crate::error::ParseError::TreeSitterError("Failed to re-parse text".to_string())
                })?;

            if self.tree.root_node().has_error()
                && has_error_in_range(self.tree.root_node(), node_start, node_start + new_len)
            {
                return Err(crate::error::ParseError::Message(format!(
                    "failed to parse include '{}'",
                    path_str
                )));
            }

            // Recurse
            let expanded_len = self.expand(actual_path, node_start, node_start + new_len)?;

            let diff = (expanded_len as isize) - ((node_end - node_start) as isize);
            end_offset = ((end_offset as isize) + diff) as usize;
            search_start = node_start + expanded_len;
        }

        self.include_stack.pop();
        Ok(end_offset - start_offset)
    }
}

fn find_first_include_in_range(node: Node, start: usize, end: usize) -> Option<Node> {
    if node.start_byte() >= end || node.end_byte() <= start {
        return None;
    }

    if node.kind() == "preproc_include" && node.start_byte() >= start && node.end_byte() <= end {
        return Some(node);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_first_include_in_range(child, start, end) {
            return Some(found);
        }
    }
    None
}

fn has_error_in_range(node: Node, start: usize, end: usize) -> bool {
    if node.start_byte() >= end || node.end_byte() <= start {
        return false;
    }
    if node.is_error() || node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_error_in_range(child, start, end) {
            return true;
        }
    }
    false
}

fn get_pos_after(start: Point, text: &str) -> Point {
    let mut row = start.row;
    let mut col = start.column;
    for b in text.as_bytes() {
        if *b == b'\n' {
            row += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Point { row, column: col }
}

fn extract_include_path(node: Node, source: &str) -> ParserResult<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "string_literal" => {
                let text = child.utf8_text(source.as_bytes())?;
                return Ok(text
                    .trim_matches(|c| c == '"' || c == '<' || c == '>')
                    .to_string());
            }
            "system_lib_string" => {
                let text = child.utf8_text(source.as_bytes())?;
                return Err(crate::error::ParseError::Message(format!(
                    "unsupported include path syntax {}; only string literal includes are supported",
                    text
                )));
            }
            "identifier" => {
                let text = child.utf8_text(source.as_bytes())?;
                return Err(crate::error::ParseError::Message(format!(
                    "unsupported include identifier '{}'; only string literal includes are supported",
                    text
                )));
            }
            _ => {}
        }
    }
    Err(crate::error::ParseError::UnexpectedNode(
        "missing include path".to_string(),
    ))
}

pub fn normalize_source_for_tree_sitter(text: &str) -> std::borrow::Cow<'_, str> {
    normalize::source(text)
}

#[cfg(test)]
mod tests;
