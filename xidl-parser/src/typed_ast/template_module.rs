use super::*;
use serde::{Deserialize, Serialize};
use xidl_parser_derive::Parser;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct TemplateModuleDcl {
    pub ident: Identifier,
    pub parameter: FormalParameters,
    #[ts(id = "tpl_definition")]
    pub definition: Vec<TplDefinition>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct TemplateModuleInst {
    pub name: ScopedName,
    pub parameter: ActualParameters,
    pub ident: Identifier,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct FormalParameters(pub Vec<FormalParameter>);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct FormalParameter {
    pub ty: FormalParameterType,
    pub ident: Identifier,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FormalParameterType {
    Typename,
    Interface,
    Valuetype,
    Eventtype,
    Struct,
    Union,
    Exception,
    Enum,
    SequenceKeyword,
    ConstType(ConstType),
    SequenceType(SequenceType),
    SimpleTypeSpec(SimpleTypeSpec),
}

impl<'a> crate::parser::FromTreeSitter<'a> for FormalParameterType {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("formal_parameter_type")
        );
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("const_type") => {
                    return Ok(Self::ConstType(crate::parser::FromTreeSitter::from_node(
                        ch, ctx,
                    )?));
                }
                xidl_parser_derive::node_id!("sequence_type") => {
                    return Ok(Self::SequenceType(
                        crate::parser::FromTreeSitter::from_node(ch, ctx)?,
                    ));
                }
                xidl_parser_derive::node_id!("simple_type_spec") => {
                    return Ok(Self::SimpleTypeSpec(
                        crate::parser::FromTreeSitter::from_node(ch, ctx)?,
                    ));
                }
                _ => {}
            }
        }

        match ctx.node_text(&node)?.trim() {
            "typename" => Ok(Self::Typename),
            "interface" => Ok(Self::Interface),
            "valuetype" => Ok(Self::Valuetype),
            "eventtype" => Ok(Self::Eventtype),
            "struct" => Ok(Self::Struct),
            "union" => Ok(Self::Union),
            "exception" => Ok(Self::Exception),
            "enum" => Ok(Self::Enum),
            "sequence" => Ok(Self::SequenceKeyword),
            _ => Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: {}",
                node.kind(),
                ctx.node_text(&node)?
            ))),
        }
    }
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum TplDefinition {
    Definition(Definition),
    TemplateModuleRef(TemplateModuleRef),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct TemplateModuleRef {
    pub name: ScopedName,
    pub parameter: FormalParameterNames,
    pub ident: Identifier,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct FormalParameterNames(pub Vec<Identifier>);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ActualParameters(pub Vec<ActualParameter>);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum ActualParameter {
    TypeSpec(TypeSpec),
    ConstExpr(ConstExpr),
}
