use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExceptDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub ident: Identifier,
    pub member: Vec<Member>,
}

impl<'a> crate::parser::FromTreeSitter<'a> for ExceptDcl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("except_dcl"));
        let mut annotations = Vec::new();
        let mut ident = None;
        let mut member = Vec::new();
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("identifier") => {
                    ident = Some(Identifier::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("member") => {
                    member.push(Member::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        if let Some(doc) = ctx.take_doc_comment(&node) {
            annotations.insert(0, AnnotationAppl::doc(doc));
        }
        Ok(Self {
            annotations,
            ident: ident.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing identifier",
                    node.kind()
                ))
            })?,
            member,
        })
    }
}
