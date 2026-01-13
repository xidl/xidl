mod base_types;
pub use base_types::*;

mod expr;
use derive::Parser;
use serde::{Deserialize, Serialize};
pub use expr::*;

mod bitmask;
pub use bitmask::*;

mod interface;
pub use interface::*;

mod union;
pub use union::*;

mod typedef_dcl_imp;
pub use typedef_dcl_imp::*;

#[derive(Debug, Parser)]
pub struct Specification(pub Vec<Definition>);

#[derive(Debug, Parser)]
pub enum Definition {
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    InterfaceDcl(InterfaceDcl),
}

#[derive(Debug, Parser)]
pub struct TypeDcl(#[ts(transparent)] pub Vec<TypeDclInner>);

#[derive(Debug, Parser)]
#[ts(transparent)]
pub enum TypeDclInner {
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug, Parser)]
pub enum ConstrTypeDcl {
    StructDcl(StructDcl),
    UnionDcl(UnionDcl),
    EnumDcl(EnumDcl),
    BitsetDcl(BitsetDcl),
    BitmaskDcl(BitmaskDcl),
}

#[derive(Debug, Parser)]
pub enum StructDcl {
    StructForwardDcl(StructForwardDcl),
    StructDef(StructDef),
}

#[derive(Debug, Parser)]
pub struct StructForwardDcl {
    pub ident: Identifier,
}

#[derive(Debug, Parser)]
pub struct StructDef {
    pub ident: Identifier,
    pub parent: Vec<ScopedName>,
    pub member: Vec<Member>,
}

#[derive(Debug, Parser)]
pub struct Member {
    pub ty: TypeSpec,
    pub ident: Declarators,
    pub default: Option<Default>,
}

#[derive(Debug, Parser)]
pub struct Default(pub ConstExpr);

#[derive(Debug, Parser)]
pub struct ConstDcl {
    pub ty: ConstType,
    pub ident: Identifier,
    pub value: ConstExpr,
}

#[derive(Debug, Parser)]
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

#[derive(Debug, Clone, PartialEq, Parser, Serialize, Deserialize)]
#[ts(transparent)]
pub struct Identifier(pub String);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct PositiveIntConst(pub ConstExpr);
