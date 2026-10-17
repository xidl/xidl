use super::*;
use serde::Serialize;
use xidl_parser_derive::Parser;

/// An `import` declaration naming the scope of another compilation unit.
///
/// The node mirrors `import_dcl` in the `tree-sitter-idl` grammar, whose
/// single child is the `imported_scope` choice.
#[derive(Debug, Parser, Serialize)]
pub struct ImportDcl {
    pub scope: ImportedScope,
}

/// The scope named by an [`ImportDcl`], mirroring `imported_scope`.
#[derive(Debug, Serialize)]
pub enum ImportedScope {
    ScopedName(ScopedName),
    StringLiteral(String),
}

impl<'a> crate::parser::FromTreeSitter<'a> for ImportedScope {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("imported_scope")
        );
        let mut scope = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("scoped_name") => {
                    scope = Some(Self::ScopedName(crate::parser::FromTreeSitter::from_node(
                        ch, ctx,
                    )?));
                }
                xidl_parser_derive::node_id!("string_literal") => {
                    scope = Some(Self::StringLiteral(ctx.node_text(&ch)?.to_string()));
                }
                _ => {}
            }
        }
        scope.ok_or_else(|| {
            crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: missing imported scope",
                node.kind()
            ))
        })
    }
}
