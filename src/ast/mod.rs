mod base_types;
pub use base_types::*;

mod expr;
pub use expr::*;

mod bitmask;
pub use bitmask::*;

mod union;
pub use union::*;

mod typedef_dcl_imp;
pub use typedef_dcl_imp::*;

#[derive(Debug)]
pub enum Definition {
    TypeDcl(TypeDcl),
}

#[derive(Debug)]
pub struct TypeDcl(pub Vec<TypeDclInner>);

#[derive(Debug)]
pub enum TypeDclInner {
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug)]
pub enum ConstrTypeDcl {
    StructDcl(StructDcl),
    UnionDcl(UnionDcl),
    EnumDcl(EnumDcl),
    BitsetDcl(BitsetDcl),
    BitmaskDcl(BitmaskDcl),
}

#[derive(Debug)]
pub enum StructDcl {
    StructForwardDcl(StructForwardDcl),
    StructDef(StructDef),
}

#[derive(Debug)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier(String);

#[derive(Debug)]
pub struct PositiveIntConst(pub ConstExpr);
