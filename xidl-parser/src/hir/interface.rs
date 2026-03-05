use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceDcl {
    pub annotations: Vec<Annotation>,
    pub decl: InterfaceDclInner,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InterfaceDclInner {
    InterfaceForwardDcl(InterfaceForwardDcl),
    InterfaceDef(InterfaceDef),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceForwardDcl {
    pub ident: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceDef {
    pub header: InterfaceHeader,
    pub interface_body: Option<InterfaceBody>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceHeader {
    pub ident: String,
    pub parent: Option<InterfaceInheritanceSpec>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceInheritanceSpec(pub Vec<InterfaceName>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceName(pub ScopedName);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceBody(pub Vec<Export>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Export {
    OpDcl(OpDcl),
    AttrDcl(AttrDcl),
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    ExceptDcl(ExceptDcl),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpDcl {
    pub annotations: Vec<Annotation>,
    pub ty: OpTypeSpec,
    pub ident: String,
    pub parameter: Option<ParameterDcls>,
    pub raises: Option<RaisesExpr>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum OpTypeSpec {
    Void,
    TypeSpec(TypeSpec),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParameterDcls(pub Vec<ParamDcl>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParamDcl {
    pub annotations: Vec<Annotation>,
    pub attr: Option<ParamAttribute>,
    pub ty: TypeSpec,
    pub declarator: SimpleDeclarator,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParamAttribute(pub String);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RaisesExpr(pub Vec<ScopedName>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttrDcl {
    pub annotations: Vec<Annotation>,
    pub decl: AttrDclInner,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AttrDclInner {
    ReadonlyAttrSpec(ReadonlyAttrSpec),
    AttrSpec(AttrSpec),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadonlyAttrSpec {
    pub ty: TypeSpec,
    pub declarator: ReadonlyAttrDeclarator,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReadonlyAttrDeclarator {
    SimpleDeclarator(SimpleDeclarator),
    RaisesExpr(RaisesExpr),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttrSpec {
    pub ty: TypeSpec,
    pub declarator: AttrDeclarator,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AttrDeclarator {
    SimpleDeclarator(Vec<SimpleDeclarator>),
    WithRaises {
        declarator: SimpleDeclarator,
        raises: AttrRaisesExpr,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AttrRaisesExpr {
    Case1(GetExcepExpr, Option<SetExcepExpr>),
    SetExcepExpr(SetExcepExpr),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetExcepExpr {
    pub expr: ExceptionList,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetExcepExpr {
    pub expr: ExceptionList,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExceptionList(pub Vec<ScopedName>);

impl From<crate::typed_ast::InterfaceDcl> for InterfaceDcl {
    fn from(value: crate::typed_ast::InterfaceDcl) -> Self {
        Self {
            annotations: expand_annotations(value.annotations),
            decl: value.decl.into(),
        }
    }
}

impl From<crate::typed_ast::InterfaceDclInner> for InterfaceDclInner {
    fn from(value: crate::typed_ast::InterfaceDclInner) -> Self {
        match value {
            crate::typed_ast::InterfaceDclInner::InterfaceForwardDcl(forward) => {
                Self::InterfaceForwardDcl(forward.into())
            }
            crate::typed_ast::InterfaceDclInner::InterfaceDef(def) => {
                Self::InterfaceDef(def.into())
            }
        }
    }
}

impl From<crate::typed_ast::InterfaceForwardDcl> for InterfaceForwardDcl {
    fn from(value: crate::typed_ast::InterfaceForwardDcl) -> Self {
        Self {
            ident: value.ident.0,
        }
    }
}

impl From<crate::typed_ast::InterfaceDef> for InterfaceDef {
    fn from(value: crate::typed_ast::InterfaceDef) -> Self {
        Self {
            header: value.header.into(),
            interface_body: value.interface_body.map(Into::into),
        }
    }
}

impl From<crate::typed_ast::InterfaceHeader> for InterfaceHeader {
    fn from(value: crate::typed_ast::InterfaceHeader) -> Self {
        Self {
            ident: value.ident.0,
            parent: value.parent.map(Into::into),
        }
    }
}

impl From<crate::typed_ast::InterfaceInheritanceSpec> for InterfaceInheritanceSpec {
    fn from(value: crate::typed_ast::InterfaceInheritanceSpec) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

impl From<crate::typed_ast::InterfaceName> for InterfaceName {
    fn from(value: crate::typed_ast::InterfaceName) -> Self {
        Self(value.0.into())
    }
}

impl From<crate::typed_ast::InterfaceBody> for InterfaceBody {
    fn from(value: crate::typed_ast::InterfaceBody) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

impl From<crate::typed_ast::Export> for Export {
    fn from(value: crate::typed_ast::Export) -> Self {
        match value {
            crate::typed_ast::Export::OpDcl(op_dcl) => Self::OpDcl(op_dcl.into()),
            crate::typed_ast::Export::AttrDcl(attr_dcl) => Self::AttrDcl(attr_dcl.into()),
            crate::typed_ast::Export::TypeDcl(type_dcl) => Self::TypeDcl(type_dcl.into()),
            crate::typed_ast::Export::ConstDcl(const_dcl) => Self::ConstDcl(const_dcl.into()),
            crate::typed_ast::Export::ExceptDcl(except_dcl) => Self::ExceptDcl(except_dcl.into()),
        }
    }
}

impl From<crate::typed_ast::OpDcl> for OpDcl {
    fn from(value: crate::typed_ast::OpDcl) -> Self {
        Self {
            annotations: expand_annotations(value.annotations),
            ty: value.ty.into(),
            ident: value.ident.0,
            parameter: value.parameter.map(Into::into),
            raises: value.raises.map(Into::into),
        }
    }
}

impl From<crate::typed_ast::OpTypeSpec> for OpTypeSpec {
    fn from(value: crate::typed_ast::OpTypeSpec) -> Self {
        match value {
            crate::typed_ast::OpTypeSpec::Void => Self::Void,
            crate::typed_ast::OpTypeSpec::TypeSpec(ty) => Self::TypeSpec(ty.into()),
        }
    }
}

impl From<crate::typed_ast::ParameterDcls> for ParameterDcls {
    fn from(value: crate::typed_ast::ParameterDcls) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

impl From<crate::typed_ast::ParamDcl> for ParamDcl {
    fn from(value: crate::typed_ast::ParamDcl) -> Self {
        Self {
            annotations: expand_annotations(value.annotations),
            attr: value.attr.map(Into::into),
            ty: value.ty.into(),
            declarator: value.declarator.into(),
        }
    }
}

impl From<crate::typed_ast::ParamAttribute> for ParamAttribute {
    fn from(value: crate::typed_ast::ParamAttribute) -> Self {
        Self(value.0)
    }
}

impl From<crate::typed_ast::RaisesExpr> for RaisesExpr {
    fn from(value: crate::typed_ast::RaisesExpr) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

impl From<crate::typed_ast::AttrDcl> for AttrDcl {
    fn from(value: crate::typed_ast::AttrDcl) -> Self {
        Self {
            annotations: expand_annotations(value.annotations),
            decl: value.decl.into(),
        }
    }
}

impl From<crate::typed_ast::AttrDclInner> for AttrDclInner {
    fn from(value: crate::typed_ast::AttrDclInner) -> Self {
        match value {
            crate::typed_ast::AttrDclInner::ReadonlyAttrSpec(spec) => {
                Self::ReadonlyAttrSpec(spec.into())
            }
            crate::typed_ast::AttrDclInner::AttrSpec(spec) => Self::AttrSpec(spec.into()),
        }
    }
}

impl From<crate::typed_ast::ReadonlyAttrSpec> for ReadonlyAttrSpec {
    fn from(value: crate::typed_ast::ReadonlyAttrSpec) -> Self {
        Self {
            ty: value.ty.into(),
            declarator: value.declarator.into(),
        }
    }
}

impl From<crate::typed_ast::ReadonlyAttrDeclarator> for ReadonlyAttrDeclarator {
    fn from(value: crate::typed_ast::ReadonlyAttrDeclarator) -> Self {
        match value {
            crate::typed_ast::ReadonlyAttrDeclarator::SimpleDeclarator(declarator) => {
                Self::SimpleDeclarator(declarator.into())
            }
            crate::typed_ast::ReadonlyAttrDeclarator::RaisesExpr(raises_expr) => {
                Self::RaisesExpr(raises_expr.into())
            }
        }
    }
}

impl From<crate::typed_ast::AttrSpec> for AttrSpec {
    fn from(value: crate::typed_ast::AttrSpec) -> Self {
        Self {
            ty: value.type_spec.into(),
            declarator: value.declarator.into(),
        }
    }
}

impl From<crate::typed_ast::AttrDeclarator> for AttrDeclarator {
    fn from(value: crate::typed_ast::AttrDeclarator) -> Self {
        match value {
            crate::typed_ast::AttrDeclarator::SimpleDeclarator(declarator) => {
                Self::SimpleDeclarator(declarator.into_iter().map(Into::into).collect())
            }
            crate::typed_ast::AttrDeclarator::WithRaises { declarator, raises } => {
                Self::WithRaises {
                    declarator: declarator.into(),
                    raises: raises.into(),
                }
            }
        }
    }
}

impl From<crate::typed_ast::AttrRaisesExpr> for AttrRaisesExpr {
    fn from(value: crate::typed_ast::AttrRaisesExpr) -> Self {
        match value {
            crate::typed_ast::AttrRaisesExpr::Case1(get, set) => {
                Self::Case1(get.into(), set.map(Into::into))
            }
            crate::typed_ast::AttrRaisesExpr::SetExcepExpr(set) => Self::SetExcepExpr(set.into()),
        }
    }
}

impl From<crate::typed_ast::GetExcepExpr> for GetExcepExpr {
    fn from(value: crate::typed_ast::GetExcepExpr) -> Self {
        Self {
            expr: value.expr.into(),
        }
    }
}

impl From<crate::typed_ast::SetExcepExpr> for SetExcepExpr {
    fn from(value: crate::typed_ast::SetExcepExpr) -> Self {
        Self {
            expr: value.expr.into(),
        }
    }
}

impl From<crate::typed_ast::ExceptionList> for ExceptionList {
    fn from(value: crate::typed_ast::ExceptionList) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}
