use super::super::{
    AutoIdKind, DataRepresentationKind, ExtensibilityKind, PlacementKind, ServicePlatform,
    TopicPlatform, TryConstructFailAction, VerbatimLanguage,
};
use crate::parser::FromTreeSitter;

impl<'a> FromTreeSitter<'a> for AutoIdKind {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "SEQUENTIAL" => Self::Sequential, "HASH" => Self::Hash })
    }
}

impl<'a> FromTreeSitter<'a> for ExtensibilityKind {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, {
            "FINAL" => Self::Final,
            "APPENDABLE" => Self::Appendable,
            "MUTABLE" => Self::Mutable
        })
    }
}

impl<'a> FromTreeSitter<'a> for VerbatimLanguage {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        match ctx.node_text(&node)?.to_ascii_lowercase().as_str() {
            "c" => Ok(Self::C),
            "c++" => Ok(Self::Cpp),
            "java" => Ok(Self::Java),
            "idl" => Ok(Self::Idl),
            "*" => Ok(Self::Any),
            value => Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: {}",
                node.kind(),
                value
            ))),
        }
    }
}

impl<'a> FromTreeSitter<'a> for PlacementKind {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, {
            "BEGIN_FILE" => Self::BeginFile,
            "BEFORE_DECLARATION" => Self::BeforeDeclaration,
            "BEGIN_DECLARATION" => Self::BeginDeclaration,
            "END_DECLARATION" => Self::EndDeclaration,
            "AFTER_DECLARATION" => Self::AfterDeclaration,
            "END_FILE" => Self::EndFile
        })
    }
}

impl<'a> FromTreeSitter<'a> for ServicePlatform {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, {
            "CORBA" => Self::Corba,
            "DDS" => Self::Dds,
            "*" => Self::Any
        })
    }
}

impl<'a> FromTreeSitter<'a> for TryConstructFailAction {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, {
            "DISCARD" => Self::Discard,
            "USE_DEFAULT" => Self::UseDefault,
            "TRIM" => Self::Trim
        })
    }
}

impl<'a> FromTreeSitter<'a> for DataRepresentationKind {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "XML" => Self::Xml })
    }
}

impl<'a> FromTreeSitter<'a> for TopicPlatform {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "DDS" => Self::Dds, "*" => Self::Any })
    }
}
