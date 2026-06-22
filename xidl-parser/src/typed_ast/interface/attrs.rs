use super::{AttrDeclarator, AttrRaisesExpr, SimpleDeclarator};
use crate::parser::FromTreeSitter;

impl<'a> FromTreeSitter<'a> for AttrDeclarator {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        let mut declarator = vec![];
        let mut raises = None;

        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("simple_declarator") => {
                    declarator.push(SimpleDeclarator::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("attr_raises_expr") => {
                    raises = Some(AttrRaisesExpr::from_node(ch, ctx)?);
                }
                _ => {}
            };
        }
        if let Some(raises) = raises {
            let mut iter = declarator.into_iter();
            let declarator = iter.next().ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing declarator",
                    node.kind(),
                ))
            })?;
            if iter.next().is_some() {
                return Err(crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: extra declarator",
                    node.kind()
                )));
            }
            Ok(Self::WithRaises { declarator, raises })
        } else {
            Ok(Self::SimpleDeclarator(declarator))
        }
    }
}

impl<'a> FromTreeSitter<'a> for AttrRaisesExpr {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("attr_raises_expr")
        );
        let mut get_excep = None;
        let mut set_excep = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("get_excep_expr") => {
                    get_excep = Some(FromTreeSitter::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("set_excep_expr") => {
                    set_excep = Some(FromTreeSitter::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        if let Some(get_excep) = get_excep {
            Ok(Self::Case1(get_excep, set_excep))
        } else if let Some(set_excep) = set_excep {
            Ok(Self::SetExcepExpr(set_excep))
        } else {
            Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: missing raises",
                node.kind()
            )))
        }
    }
}
