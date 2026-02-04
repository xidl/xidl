use serde::{Deserialize, Serialize};
use xidl_parser_derive::Parser;

use crate::typed_ast::Identifier;

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct ConstExpr(pub OrExpr);

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum OrExpr {
    XorExpr(XorExpr),
    OrExpr(Box<OrExpr>, XorExpr),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum XorExpr {
    AndExpr(AndExpr),
    XorExpr(Box<XorExpr>, AndExpr),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum AndExpr {
    ShiftExpr(ShiftExpr),
    AndExpr(Box<AndExpr>, ShiftExpr),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum ShiftExpr {
    AddExpr(AddExpr),
    LeftShiftExpr(Box<ShiftExpr>, AddExpr),
    RightShiftExpr(Box<ShiftExpr>, AddExpr),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum AddExpr {
    MultExpr(MultExpr),
    AddExpr(Box<AddExpr>, MultExpr),
    SubExpr(Box<AddExpr>, MultExpr),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum MultExpr {
    UnaryExpr(UnaryExpr),
    MultExpr(Box<MultExpr>, UnaryExpr),
    DivExpr(Box<MultExpr>, UnaryExpr),
    ModExpr(Box<MultExpr>, UnaryExpr),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum UnaryExpr {
    UnaryExpr(UnaryOperator, PrimaryExpr),
    PrimaryExpr(PrimaryExpr),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum PrimaryExpr {
    ScopedName(ScopedName),
    Literal(Literal),
    ConstExpr(Box<ConstExpr>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOperator {
    Add,
    Sub,
    Not,
}

impl<'a> crate::parser::FromTreeSitter<'a> for UnaryOperator {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        #[allow(clippy::never_loop)]
        for ch in node.children(&mut node.walk()) {
            return match ctx.node_text(&ch)? {
                "+" => Ok(Self::Add),
                "-" => Ok(Self::Sub),
                "~" => Ok(Self::Not),
                _ => Err(crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: {}",
                    node.kind(),
                    ch.kind()
                ))),
            };
        }
        unreachable!()
    }
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct ScopedName {
    #[ts(id = "scoped_name")]
    pub scoped_name: Option<Box<ScopedName>>,
    pub identifier: Identifier,
    #[ts(id = "-", text)]
    pub node_text: String,
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum Literal {
    IntegerLiteral(IntegerLiteral),
    FloatingPtLiteral(FloatingPtLiteral),
    // FixedPtLiteral,
    CharLiteral(String),
    WideCharacterLiteral(String),
    StringLiteral(String),
    WideStringLiteral(String),
    BooleanLiteral(String),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum IntegerLiteral {
    BinNumber(String),
    OctNumber(String),
    DecNumber(String),
    HexNumber(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingPtLiteral {
    pub sign: Option<IntegerSign>,
    pub integer: DecNumber,
    pub fraction: DecNumber,
}

impl<'a> crate::parser::FromTreeSitter<'a> for FloatingPtLiteral {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("floating_pt_literal")
        );
        let mut sign = None;
        let mut integer = None;
        let mut fraction = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("integer_sign") => {
                    sign = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("dec_number") => {
                    let inter = crate::parser::FromTreeSitter::from_node(ch, ctx)?;
                    if integer.is_none() {
                        integer = Some(inter);
                    } else {
                        fraction = Some(inter);
                    }
                }

                _ => {}
            }
        }
        Ok(Self {
            sign,
            integer: integer.unwrap(),
            fraction: fraction.unwrap(),
        })
    }
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
#[ts(transparent)]
pub struct IntegerSign(pub String);

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
#[ts(transparent)]
pub struct DecNumber(pub String);
