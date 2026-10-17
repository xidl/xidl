//! Semantic interface, operation, and attribute declarations.

mod convert;

use super::*;
use serde::{Deserialize, Serialize};

/// An interface declaration with its contract annotations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceDcl {
    pub annotations: Vec<Annotation>,
    pub decl: InterfaceDclInner,
}

/// A complete interface definition or forward declaration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InterfaceDclInner {
    InterfaceForwardDcl(InterfaceForwardDcl),
    InterfaceDef(InterfaceDef),
}

/// An interface name declared before its definition.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceForwardDcl {
    pub ident: String,
}

/// An interface header and its optional body.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceDef {
    pub header: InterfaceHeader,
    pub interface_body: Option<InterfaceBody>,
}

/// An interface name and inherited interfaces.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceHeader {
    pub ident: String,
    pub parent: Option<InterfaceInheritanceSpec>,
}

/// The parent interfaces named by an inheritance clause.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceInheritanceSpec(pub Vec<InterfaceName>);

/// A scoped reference to an interface.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceName(pub ScopedName);

/// Declarations exported by an interface.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceBody(pub Vec<Export>);

/// An operation, attribute, type, constant, or exception in an interface.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Export {
    OpDcl(OpDcl),
    AttrDcl(AttrDcl),
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    ExceptDcl(ExceptDcl),
}

/// An operation signature with annotations, parameters, and exceptions.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpDcl {
    pub annotations: Vec<Annotation>,
    pub ty: OpTypeSpec,
    pub ident: String,
    pub parameter: Option<ParameterDcls>,
    pub raises: Option<RaisesExpr>,
}

/// An operation return type, including void.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum OpTypeSpec {
    Void,
    TypeSpec(TypeSpec),
}

/// The ordered parameters of an operation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParameterDcls(pub Vec<ParamDcl>);

/// An operation parameter and its binding annotations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParamDcl {
    pub annotations: Vec<Annotation>,
    pub attr: Option<ParamAttribute>,
    pub ty: TypeSpec,
    pub declarator: SimpleDeclarator,
}

/// A parameter direction such as in, out, or inout.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParamAttribute(pub String);

/// The ordered exceptions declared by an operation or getter.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RaisesExpr(pub Vec<ScopedName>);

/// An interface attribute with contract annotations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttrDcl {
    pub annotations: Vec<Annotation>,
    pub decl: AttrDclInner,
}

/// A readonly or writable attribute specification.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AttrDclInner {
    ReadonlyAttrSpec(ReadonlyAttrSpec),
    AttrSpec(AttrSpec),
}

/// The type and names of a readonly attribute declaration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadonlyAttrSpec {
    pub ty: TypeSpec,
    pub declarator: ReadonlyAttrDeclarator,
}

/// Readonly attribute names and the raises clause belonging to one name.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReadonlyAttrDeclarator {
    SimpleDeclarator(Vec<SimpleDeclarator>),
    WithRaises {
        declarator: SimpleDeclarator,
        raises: RaisesExpr,
    },
}

impl ReadonlyAttrDeclarator {
    /// All attribute names, including the name with a raises clause.
    pub fn declarators(&self) -> &[SimpleDeclarator] {
        match self {
            Self::SimpleDeclarator(names) => names,
            Self::WithRaises { declarator, .. } => std::slice::from_ref(declarator),
        }
    }

    /// Exceptions declared for the attribute getter.
    pub fn raises(&self) -> Option<&RaisesExpr> {
        match self {
            Self::SimpleDeclarator(_) => None,
            Self::WithRaises { raises, .. } => Some(raises),
        }
    }

    /// Exceptions to rewrite when projecting the getter into a version.
    pub fn raises_mut(&mut self) -> Option<&mut RaisesExpr> {
        match self {
            Self::SimpleDeclarator(_) => None,
            Self::WithRaises { raises, .. } => Some(raises),
        }
    }
}

/// The type, names, and exception clauses of writable attributes.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttrSpec {
    pub ty: TypeSpec,
    pub declarator: AttrDeclarator,
}

/// Writable attribute names and any getter or setter exceptions.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AttrDeclarator {
    SimpleDeclarator(Vec<SimpleDeclarator>),
    WithRaises {
        declarator: SimpleDeclarator,
        raises: AttrRaisesExpr,
    },
}

/// Getter and setter exception clauses for one attribute.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AttrRaisesExpr {
    Case1(GetExcepExpr, Option<SetExcepExpr>),
    SetExcepExpr(SetExcepExpr),
}

/// The exception list of an attribute getter.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetExcepExpr {
    pub expr: ExceptionList,
}

/// The exception list of an attribute setter.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetExcepExpr {
    pub expr: ExceptionList,
}

/// The ordered scoped exception names in an attribute clause.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExceptionList(pub Vec<ScopedName>);
