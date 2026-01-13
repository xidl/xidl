mod enum_dcl;
pub use enum_dcl::*;

mod struct_dcl;
use serde::{Deserialize, Serialize};
pub use struct_dcl::*;

mod declarator;
pub use declarator::*;

mod types;
pub use types::*;

mod const_dcl;
pub use const_dcl::*;

mod interface;
pub use interface::*;

mod type_dcl;
pub use type_dcl::*;

mod exception_dcl;
pub use exception_dcl::*;

use crate::typed_ast::{ConstExpr, PositiveIntConst};

#[derive(Debug, Serialize, Deserialize)]
pub struct Specification(pub Vec<Definition>);

#[derive(Debug, Serialize, Deserialize)]
pub enum Definition {
    ConstrTypeDcl(ConstrTypeDcl),
    ConstDcl(ConstDcl),
    InterfaceDcl(InterfaceDcl),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConstrTypeDcl {
    StructForwardDcl(StructForwardDcl),
    StructDcl(StructDcl),
    EnumDcl(EnumDcl),
    UnionForwardDcl(UnionForwardDcl),
    UnionDef(UnionDef),
    BitsetDcl(BitsetDcl),
    BitmaskDcl(BitmaskDcl),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnionForwardDcl {
    pub ident: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnionDef {
    pub ident: String,
    pub switch_type_spec: SwitchTypeSpec,
    pub case: Vec<Case>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Case {
    pub label: Vec<ConstExpr>,
    pub element: ElementSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementSpec {
    pub ty: ElementSpecTy,
    pub value: Declarator,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ElementSpecTy {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SwitchTypeSpec {
    IntegerType(IntegerType),
    CharType,
    WideCharType,
    BooleanType,
    ScopedName(ScopedName),
    OctetType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitsetDcl {
    pub ident: String,
    pub parent: Option<ScopedName>,
    pub field: Vec<BitField>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BitFieldType {
    Bool,
    Octec,
    SignedInt,
    UnsignedInt,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitField {
    pub ident: String,
    pub pos: PositiveIntConst,
    pub ty: Option<BitFieldType>,
}

#[derive(Debug, Serialize, Deserialize)]
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

impl From<crate::typed_ast::Specification> for Specification {
    fn from(value: crate::typed_ast::Specification) -> Self {
        let mut defs = Vec::new();
        for def in value.0 {
            match def {
                crate::typed_ast::Definition::TypeDcl(type_dcl) => {
                    for inner in type_dcl.0 {
                        if let crate::typed_ast::TypeDclInner::ConstrTypeDcl(constr) = inner {
                            defs.push(Definition::ConstrTypeDcl(constr.into()));
                        }
                    }
                }
                crate::typed_ast::Definition::ConstDcl(const_dcl) => {
                    defs.push(Definition::ConstDcl(const_dcl.into()));
                }
                crate::typed_ast::Definition::InterfaceDcl(interface_dcl) => {
                    defs.push(Definition::InterfaceDcl(interface_dcl.into()));
                }
                crate::typed_ast::Definition::ModuleDcl(_)
                | crate::typed_ast::Definition::ExceptDcl(_)
                | crate::typed_ast::Definition::TemplateModuleDcl(_)
                | crate::typed_ast::Definition::TemplateModuleInst(_)
                | crate::typed_ast::Definition::PreprocInclude(_)
                | crate::typed_ast::Definition::PreprocCall(_)
                | crate::typed_ast::Definition::PreprocDefine(_) => {}
            }
        }
        Self(defs)
    }
}

impl From<crate::typed_ast::ConstrTypeDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::ConstrTypeDcl) -> Self {
        match value {
            crate::typed_ast::ConstrTypeDcl::StructDcl(struct_dcl) => struct_dcl.into(),
            crate::typed_ast::ConstrTypeDcl::UnionDcl(union_dcl) => union_dcl.into(),
            crate::typed_ast::ConstrTypeDcl::EnumDcl(enum_dcl) => Self::EnumDcl(enum_dcl.into()),
            crate::typed_ast::ConstrTypeDcl::BitsetDcl(bitset_dcl) => {
                Self::BitsetDcl(bitset_dcl.into())
            }
            crate::typed_ast::ConstrTypeDcl::BitmaskDcl(bitmask_dcl) => {
                Self::BitmaskDcl(bitmask_dcl.into())
            }
        }
    }
}

impl From<crate::typed_ast::StructDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::StructDcl) -> Self {
        match value {
            crate::typed_ast::StructDcl::StructForwardDcl(forward) => {
                Self::StructForwardDcl(forward.into())
            }
            crate::typed_ast::StructDcl::StructDef(def) => Self::StructDcl(def.into()),
        }
    }
}

impl From<crate::typed_ast::UnionDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::UnionDcl) -> Self {
        match value {
            crate::typed_ast::UnionDcl::UnionForwardDcl(forward) => {
                Self::UnionForwardDcl(forward.into())
            }
            crate::typed_ast::UnionDcl::UnionDef(def) => Self::UnionDef(def.into()),
        }
    }
}

impl From<crate::typed_ast::UnionForwardDcl> for UnionForwardDcl {
    fn from(value: crate::typed_ast::UnionForwardDcl) -> Self {
        Self { ident: value.0 .0 }
    }
}

impl From<crate::typed_ast::UnionDef> for UnionDef {
    fn from(value: crate::typed_ast::UnionDef) -> Self {
        Self {
            ident: value.ident.0,
            switch_type_spec: value.switch_type_spec.into(),
            case: value.case.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::typed_ast::Case> for Case {
    fn from(value: crate::typed_ast::Case) -> Self {
        Self {
            label: value.label.into_iter().map(|label| label.0).collect(),
            element: value.element.into(),
        }
    }
}

impl From<crate::typed_ast::ElementSpec> for ElementSpec {
    fn from(value: crate::typed_ast::ElementSpec) -> Self {
        Self {
            ty: value.ty.into(),
            value: value.value.into(),
        }
    }
}

impl From<crate::typed_ast::ElementSpecTy> for ElementSpecTy {
    fn from(value: crate::typed_ast::ElementSpecTy) -> Self {
        match value {
            crate::typed_ast::ElementSpecTy::TypeSpec(ty) => Self::TypeSpec(ty.into()),
            crate::typed_ast::ElementSpecTy::ConstrTypeDcl(constr) => {
                Self::ConstrTypeDcl(constr.into())
            }
        }
    }
}

impl From<crate::typed_ast::SwitchTypeSpec> for SwitchTypeSpec {
    fn from(value: crate::typed_ast::SwitchTypeSpec) -> Self {
        match value {
            crate::typed_ast::SwitchTypeSpec::IntegerType(integer_type) => {
                Self::IntegerType(integer_type.into())
            }
            crate::typed_ast::SwitchTypeSpec::CharType(_) => Self::CharType,
            crate::typed_ast::SwitchTypeSpec::WideCharType(_) => Self::WideCharType,
            crate::typed_ast::SwitchTypeSpec::BooleanType(_) => Self::BooleanType,
            crate::typed_ast::SwitchTypeSpec::ScopedName(scoped_name) => {
                Self::ScopedName(scoped_name.into())
            }
            crate::typed_ast::SwitchTypeSpec::OctetType(_) => Self::OctetType,
        }
    }
}

impl From<crate::typed_ast::BitsetDcl> for BitsetDcl {
    fn from(value: crate::typed_ast::BitsetDcl) -> Self {
        let mut field = Vec::new();
        for bitfield in value.field {
            let pos = bitfield.spec.pos;
            let ty = bitfield.spec.dst_ty.map(Into::into);
            for ident in bitfield.ident {
                field.push(BitField {
                    ident: ident.0,
                    pos: pos.clone(),
                    ty: ty.clone(),
                });
            }
        }

        Self {
            ident: value.ident.0,
            parent: value.parent.map(Into::into),
            field,
        }
    }
}

impl From<crate::typed_ast::DestinationType> for BitFieldType {
    fn from(value: crate::typed_ast::DestinationType) -> Self {
        match value {
            crate::typed_ast::DestinationType::BooleanType(_) => Self::Bool,
            crate::typed_ast::DestinationType::OctetType(_) => Self::Octec,
            crate::typed_ast::DestinationType::IntegerType(integer_type) => {
                if matches!(integer_type, crate::typed_ast::IntegerType::SignedInt(_)) {
                    Self::SignedInt
                } else {
                    Self::UnsignedInt
                }
            }
        }
    }
}

impl From<crate::typed_ast::BitmaskDcl> for BitmaskDcl {
    fn from(value: crate::typed_ast::BitmaskDcl) -> Self {
        let mut entries = Vec::new();
        for bit_value in value.value {
            for ident in bit_value.0 {
                entries.push(ident.0);
            }
        }
        Self {
            ident: value.ident.0,
            value: entries,
        }
    }
}
