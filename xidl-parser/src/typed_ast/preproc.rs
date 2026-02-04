use super::*;
use serde::{Deserialize, Serialize};
use xidl_parser_derive::Parser;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct PreprocDefine {
    #[ts(id = "args", text)]
    pub args: String,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct PreprocCall {
    pub directive: PreprocDirective,
    pub argument: Option<PreprocArg>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(transparent)]
pub struct PreprocDirective(pub String);

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(transparent)]
pub struct PreprocArg(pub String);

#[derive(Debug, Serialize, Deserialize)]
pub struct PreprocInclude {
    pub path: PreprocIncludePath,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PreprocIncludePath {
    StringLiteral(String),
    SystemLibString(String),
    Identifier(Identifier),
}

impl<'a> crate::parser::FromTreeSitter<'a> for PreprocInclude {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("preproc_include")
        );
        let mut path = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("string_literal") => {
                    path = Some(PreprocIncludePath::StringLiteral(
                        ctx.node_text(&ch)?.to_string(),
                    ));
                }
                xidl_parser_derive::node_id!("system_lib_string") => {
                    path = Some(PreprocIncludePath::SystemLibString(
                        ctx.node_text(&ch)?.to_string(),
                    ));
                }
                xidl_parser_derive::node_id!("identifier") => {
                    path = Some(PreprocIncludePath::Identifier(
                        crate::parser::FromTreeSitter::from_node(ch, ctx)?,
                    ));
                }
                _ => {}
            }
        }
        let Some(path) = path else {
            return Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: missing include path",
                node.kind()
            )));
        };
        Ok(Self { path })
    }
}
