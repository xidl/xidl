mod base_types;
pub use base_types::*;

mod expr;
use derive::Parser;
pub use expr::*;

mod bitmask;
pub use bitmask::*;

mod union;
pub use union::*;

mod typedef_dcl_imp;
pub use typedef_dcl_imp::*;

#[derive(Debug, Parser)]
pub struct Specification(pub Vec<Definition>);

#[derive(Debug, Parser)]
pub enum Definition {
    TypeDcl(TypeDcl),
}

#[derive(Debug)]
pub struct TypeDcl(pub Vec<TypeDclInner>);

impl<'a> crate::parser::FromTreeSitter<'a> for TypeDcl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        use crate::parser::FromTreeSitter;
        assert_eq!(node.kind_id(), derive::node_id!("type_dcl"));
        let mut field_0 = vec![];
        for ch in node.children(&mut node.walk()) {
            if let Ok(node) = FromTreeSitter::from_node(ch, ctx) {
                field_0.push(node)
            }
        }

        Ok(Self(field_0))
    }
}

#[derive(Debug, Parser)]
pub enum TypeDclInner {
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug, Parser)]
pub enum ConstrTypeDcl {
    StructDcl(StructDcl),
    // UnionDcl(UnionDcl),
    // EnumDcl(EnumDcl),
    // BitsetDcl(BitsetDcl),
    // BitmaskDcl(BitmaskDcl),
}

#[derive(Debug, Parser)]
pub enum StructDcl {
    StructForwardDcl(StructForwardDcl),
    // StructDef(StructDef),
}

#[derive(Debug, Parser)]
pub struct StructForwardDcl {
    pub ident: Identifier,
}

#[derive(Debug)]
pub struct StructDef {
    pub ident: Identifier,
    pub parent: Vec<ScopedName>,
    pub member: Vec<Member>,
}

#[derive(Debug)]
pub struct Member {
    pub ty: TypeSpec,
    pub ident: Declarators,
    pub default: Option<Default>,
}

#[derive(Debug)]
pub struct Default(pub ConstExpr);

pub struct ConstDcl {
    pub ty: ConstType,
    pub ident: Identifier,
    pub value: ConstExpr,
}

pub enum ConstType {
    IntegerType(IntegerType),
    FloatingPtType(FloatingPtType),
    FixedPtConstType(FixedPtConstType),
    CharType(CharType),
    WideCharType(WideCharType),
    BooleanType(BooleanType),
    OctetType(OctetType),
    StringType(StringType),
    WideStringType(WideStringType),
    ScopedName(ScopedName),
    SequenceType(SequenceType),
}

#[derive(Debug, Clone, PartialEq, Parser)]
#[ts(text)]
pub struct Identifier(String);

#[derive(Debug)]
pub struct PositiveIntConst(pub ConstExpr);
