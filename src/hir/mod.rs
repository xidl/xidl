mod enum_dcl;
pub use enum_dcl::*;

mod struct_dcl;
use serde::{Deserialize, Serialize};
pub use struct_dcl::*;

mod declarator;
pub use declarator::*;

mod types;
pub use types::*;

use crate::typed_ast::{ConstExpr, PositiveIntConst};

pub struct Specification(pub Vec<Definition>);

pub enum Definition {
    ConstrTypeDcl(ConstrTypeDcl),
}

pub enum ConstrTypeDcl {
    StructForwardDcl(StructForwardDcl),
    StructDcl(StructDcl),
    EnumDcl(EnumDcl),
    UnionForwardDcl(UnionForwardDcl),
    UnionDef(UnionDef),
    BitsetDcl(BitsetDcl),
    BitmaskDcl(BitmaskDcl),
}

pub struct UnionForwardDcl {
    pub ident: String,
}

pub struct UnionDef {
    pub ident: String,
    pub switch_type_spec: SwitchTypeSpec,
    pub case: Vec<Case>,
}

pub struct Case {
    pub label: Vec<ConstExpr>,
    pub element: ElementSpec,
}

pub struct ElementSpec {
    pub ty: ElementSpecTy,
    pub value: Declarator,
}

pub enum ElementSpecTy {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

pub enum SwitchTypeSpec {
    IntegerType(IntegerType),
    CharType,
    WideCharType,
    BooleanType,
    ScopedName(ScopedName),
    OctetType,
}

pub struct BitsetDcl {
    pub ident: String,
    pub parent: Option<ScopedName>,
    pub field: Vec<BitField>,
    pub ty: BitFieldType,
}

pub enum BitFieldType {
    Bool,
    Octec,
    SignedInt,
    UnsignedInt,
}

pub struct BitField {
    pub ident: String,
    pub pos: PositiveIntConst,
}

pub struct BitmaskDcl {
    pub ident: String,
    pub value: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScopedName {
    pub name: Vec<String>,
    pub is_root: bool,
}

impl From<crate::typed_ast::ScopedName> for ScopedName {
    fn from(typed_ast: crate::typed_ast::ScopedName) -> Self {
        let is_root = typed_ast.node_text.starts_with("::");
        let mut v = vec![];
        get_scoped_name(&mut v, &typed_ast);
        let name = v.into_iter().map(ToOwned::to_owned).collect();

        Self { name, is_root }
    }
}

fn get_scoped_name<'a>(pre: &mut Vec<&'a str>, value: &'a crate::typed_ast::ScopedName) {
    if let Some(value) = &value.scoped_name {
        get_scoped_name(pre, value);
    }

    pre.push(&value.identifier.0);
}
