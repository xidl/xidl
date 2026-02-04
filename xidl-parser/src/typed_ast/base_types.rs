use serde::{Deserialize, Serialize};
use xidl_parser_derive::Parser;

use super::*;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(mark)]
pub struct SignedShortInt;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(mark)]
pub struct SignedLongInt;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(id = "signed_longlong_int")]
pub struct SignedLongLongInt;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum UnsignedInt {
    UnsignedShortInt(UnsignedShortInt),
    UnsignedLongInt(UnsignedLongInt),
    #[ts(id = "unsigned_longlong_int")]
    UnsignedLongLongInt(UnsignedLongLongInt),
    UnsignedTinyInt(UnsignedTinyInt),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(mark)]
pub struct UnsignedTinyInt;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(name = "boolean")]
pub struct BooleanType;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(name = "fixed")]
pub struct FixedPtConstType;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(name = "octet")]
pub struct OctetType;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum IntegerType {
    SignedInt(SignedInt),
    UnsignedInt(UnsignedInt),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum SignedInt {
    SignedShortInt(SignedShortInt),
    SignedLongInt(SignedLongInt),
    #[ts(id = "signed_longlong_int")]
    SignedLongLongInt(SignedLongLongInt),
    SignedTinyInt(SignedTinyInt),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(name = "int8")]
pub struct SignedTinyInt;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(mark)]
pub struct UnsignedShortInt;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(mark)]
pub struct UnsignedLongInt;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(mark)]
#[ts(id = "unsigned_longlong_int")]
pub struct UnsignedLongLongInt;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(mark)]
pub struct FloatingPtType;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(name = "char")]
pub struct CharType;

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(name = "wchar")]
pub struct WideCharType;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct StringType {
    pub bound: Option<PositiveIntConst>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct WideStringType {
    pub bound: Option<PositiveIntConst>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum TypeSpec {
    SimpleTypeSpec(SimpleTypeSpec),
    TemplateTypeSpec(TemplateTypeSpec),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum SimpleTypeSpec {
    BaseTypeSpec(BaseTypeSpec),
    ScopedName(ScopedName),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum BaseTypeSpec {
    IntegerType(IntegerType),
    FloatingPtType(FloatingPtType),
    CharType(CharType),
    WideCharType(WideCharType),
    BooleanType(BooleanType),
    OctetType(OctetType),
    AnyType(AnyType),
    ObjectType(ObjectType),
    ValueBaseType(ValueBaseType),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(name = "any")]
pub struct AnyType;

#[derive(Debug, Serialize, Deserialize)]
pub struct FixedPtType {
    pub integer: PositiveIntConst,
    pub fraction: PositiveIntConst,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum TemplateTypeSpec {
    SequenceType(SequenceType),
    StringType(StringType),
    WideStringType(WideStringType),
    FixedPtType(FixedPtType),
    MapType(MapType),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct SequenceType {
    pub ty: Box<TypeSpec>,
    pub len: Option<PositiveIntConst>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MapType {
    pub key: Box<TypeSpec>,
    pub value: Box<TypeSpec>,
    pub len: Option<PositiveIntConst>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ObjectType;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ValueBaseType;

impl<'a> crate::parser::FromTreeSitter<'a> for FixedPtType {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("fixed_pt_type")
        );
        let mut values = Vec::new();
        for ch in node.children(&mut node.walk()) {
            if ch.kind_id() == xidl_parser_derive::node_id!("positive_int_const") {
                values.push(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
            }
        }
        let mut iter = values.into_iter();
        let Some(integer) = iter.next() else {
            return Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: missing integer",
                node.kind()
            )));
        };
        let Some(fraction) = iter.next() else {
            return Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: missing fraction",
                node.kind()
            )));
        };
        Ok(Self { integer, fraction })
    }
}

impl<'a> crate::parser::FromTreeSitter<'a> for MapType {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("map_type"));
        let mut types = Vec::new();
        let mut len = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("type_spec") => {
                    types.push(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("positive_int_const") => {
                    len = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        let mut iter = types.into_iter();
        let Some(key) = iter.next() else {
            return Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: missing key type",
                node.kind()
            )));
        };
        let Some(value) = iter.next() else {
            return Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: missing value type",
                node.kind()
            )));
        };
        Ok(Self {
            key: Box::new(key),
            value: Box::new(value),
            len,
        })
    }
}
