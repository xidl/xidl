use crate::typed_ast::ScopedName;
use serde::{Deserialize, Serialize};

use super::{AnnotationAppl, ConstDcl, ExceptDcl, Identifier, SimpleDeclarator, TypeDcl, TypeSpec};
use xidl_parser_derive::Parser;

#[derive(Debug, Serialize, Deserialize)]
pub struct InterfaceDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub decl: InterfaceDclInner,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(transparent)]
pub enum InterfaceDclInner {
    InterfaceForwardDcl(InterfaceForwardDcl),
    InterfaceDef(InterfaceDef),
}

impl<'a> crate::parser::FromTreeSitter<'a> for InterfaceDcl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("interface_dcl")
        );
        let mut annotations = vec![];
        let mut decl = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("interface_def")
                | xidl_parser_derive::node_id!("interface_forward_dcl") => {
                    decl = Some(InterfaceDclInner::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            annotations,
            decl: decl.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing interface decl",
                    node.kind()
                ))
            })?,
        })
    }
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct InterfaceForwardDcl {
    pub kind: InterfaceKind,
    pub ident: Identifier,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(mark)]
pub struct InterfaceKind;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct InterfaceDef {
    pub header: InterfaceHeader,
    pub interface_body: Option<InterfaceBody>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct InterfaceHeader {
    pub kind: InterfaceKind,
    pub ident: Identifier,
    pub parent: Option<InterfaceInheritanceSpec>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct InterfaceInheritanceSpec(pub Vec<InterfaceName>);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct InterfaceName(pub ScopedName);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct InterfaceBody(pub Vec<Export>);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum Export {
    OpDcl(OpDcl),
    AttrDcl(AttrDcl),
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    ExceptDcl(ExceptDcl),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub ty: OpTypeSpec,
    pub ident: Identifier,
    pub parameter: Option<ParameterDcls>,
    pub raises: Option<RaisesExpr>,
}

impl<'a> crate::parser::FromTreeSitter<'a> for OpDcl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("op_dcl"));
        let mut annotations = Vec::new();
        let mut ty = None;
        let mut ident = None;
        let mut parameter = None;
        let mut raises = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("op_type_spec") => {
                    ty = Some(OpTypeSpec::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("identifier") => {
                    ident = Some(Identifier::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("parameter_dcls") => {
                    parameter = Some(ParameterDcls::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("raises_expr") => {
                    raises = Some(RaisesExpr::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            annotations,
            ty: ty.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing op type",
                    node.kind()
                ))
            })?,
            ident: ident.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing identifier",
                    node.kind()
                ))
            })?,
            parameter,
            raises,
        })
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum OpTypeSpec {
    Void,
    TypeSpec(TypeSpec),
}

impl<'a> crate::parser::FromTreeSitter<'a> for OpTypeSpec {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        #[allow(clippy::never_loop)]
        for ch in node.children(&mut node.walk()) {
            if ctx.node_text(&ch)? == "void" {
                return Ok(Self::Void);
            }

            return match ch.kind_id() {
                xidl_parser_derive::node_id!("type_spec") => Ok(Self::TypeSpec(
                    crate::parser::FromTreeSitter::from_node(ch, ctx)?,
                )),
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

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ParameterDcls(pub Vec<ParamDcl>);

#[derive(Debug, Serialize, Deserialize)]
pub struct ParamDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub attr: Option<ParamAttribute>,
    pub ty: TypeSpec,
    pub declarator: SimpleDeclarator,
}

impl<'a> crate::parser::FromTreeSitter<'a> for ParamDcl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("param_dcl"));
        let mut annotations = Vec::new();
        let mut attr = None;
        let mut ty = None;
        let mut declarator = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("param_attribute") => {
                    attr = Some(ParamAttribute::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("type_spec") => {
                    ty = Some(TypeSpec::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("simple_declarator") => {
                    declarator = Some(SimpleDeclarator::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            annotations,
            attr,
            ty: ty.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing type spec",
                    node.kind()
                ))
            })?,
            declarator: declarator.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing simple declarator",
                    node.kind()
                ))
            })?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParamAttribute(pub String);

impl<'a> crate::parser::FromTreeSitter<'a> for ParamAttribute {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("param_attribute")
        );

        Ok(Self(ctx.node_text(&node)?.to_string()))
    }
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct RaisesExpr(pub Vec<ScopedName>);

#[derive(Debug, Serialize, Deserialize)]
pub struct AttrDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub decl: AttrDclInner,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(transparent)]
pub enum AttrDclInner {
    ReadonlyAttrSpec(ReadonlyAttrSpec),
    AttrSpec(AttrSpec),
}

impl<'a> crate::parser::FromTreeSitter<'a> for AttrDcl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("attr_dcl"));
        let mut annotations = vec![];
        let mut decl = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("readonly_attr_spec")
                | xidl_parser_derive::node_id!("attr_spec") => {
                    decl = Some(AttrDclInner::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            annotations,
            decl: decl.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing attr decl",
                    node.kind()
                ))
            })?,
        })
    }
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ReadonlyAttrSpec {
    pub ty: TypeSpec,
    pub declarator: ReadonlyAttrDeclarator,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum ReadonlyAttrDeclarator {
    SimpleDeclarator(SimpleDeclarator),
    RaisesExpr(RaisesExpr),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct AttrSpec {
    pub type_spec: TypeSpec,
    pub declarator: AttrDeclarator,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AttrDeclarator {
    SimpleDeclarator(Vec<SimpleDeclarator>),
    WithRaises {
        declarator: SimpleDeclarator,
        raises: AttrRaisesExpr,
    },
}

impl<'a> crate::parser::FromTreeSitter<'a> for AttrDeclarator {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        let mut declarator = vec![];
        let mut raises = None;

        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("simple_declarator") => {
                    declarator.push(SimpleDeclarator::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("attr_raises_expr") => {
                    raises = Some(AttrRaisesExpr::from_node(ch, ctx)?);
                }
                _ => {}
            };
        }
        if let Some(raises) = raises {
            let mut iter = declarator.into_iter();
            let declarator = iter.next().ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing declarator",
                    node.kind(),
                ))
            })?;
            if iter.next().is_some() {
                return Err(crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: extra declarator",
                    node.kind()
                )));
            }
            Ok(Self::WithRaises { declarator, raises })
        } else {
            Ok(Self::SimpleDeclarator(declarator))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AttrRaisesExpr {
    Case1(GetExcepExpr, Option<SetExcepExpr>),
    SetExcepExpr(SetExcepExpr),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct GetExcepExpr {
    pub expr: ExceptionList,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct SetExcepExpr {
    pub expr: ExceptionList,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ExceptionList(pub Vec<ScopedName>);

impl<'a> crate::parser::FromTreeSitter<'a> for AttrRaisesExpr {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("attr_raises_expr")
        );
        let mut get_excep = None;
        let mut set_excep = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("get_excep_expr") => {
                    get_excep = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("set_excep_expr") => {
                    set_excep = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        if let Some(get_excep) = get_excep {
            Ok(Self::Case1(get_excep, set_excep))
        } else if let Some(set_excep) = set_excep {
            Ok(Self::SetExcepExpr(set_excep))
        } else {
            Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: missing raises",
                node.kind()
            )))
        }
    }
}
