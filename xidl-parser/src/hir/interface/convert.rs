//! Lowering concrete interface nodes into semantic declarations.

use super::*;

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
            crate::typed_ast::ReadonlyAttrDeclarator::SimpleDeclarator(declarators) => {
                Self::SimpleDeclarator(declarators.into_iter().map(Into::into).collect())
            }
            crate::typed_ast::ReadonlyAttrDeclarator::WithRaises { declarator, raises } => {
                Self::WithRaises {
                    declarator: declarator.into(),
                    raises: raises.into(),
                }
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
