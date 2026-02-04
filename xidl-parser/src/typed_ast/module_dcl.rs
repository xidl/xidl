use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub ident: Identifier,
    pub definition: Vec<Definition>,
}

impl<'a> crate::parser::FromTreeSitter<'a> for ModuleDcl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("module_dcl"));
        let mut annotations = Vec::new();
        let mut ident = None;
        let mut definition = Vec::new();
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("identifier") => {
                    ident = Some(Identifier::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("definition") => {
                    definition.push(Definition::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            annotations,
            ident: ident.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing identifier",
                    node.kind()
                ))
            })?,
            definition,
        })
    }
}
