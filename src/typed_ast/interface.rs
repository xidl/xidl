use crate::typed_ast::ScopedName;

use super::{ConstDcl, ExceptDcl, Identifier, SimpleDeclarator, TypeDcl, TypeSpec};
use derive::Parser;

#[derive(Debug, Parser)]
pub enum InterfaceDcl {
    InterfaceForwardDcl(InterfaceForwardDcl),
    InterfaceDef(InterfaceDef),
}

#[derive(Debug, Parser)]
pub struct InterfaceForwardDcl {
    pub kind: InterfaceKind,
    pub ident: Identifier,
}

#[derive(Debug, Parser)]
#[ts(mark)]
pub struct InterfaceKind;

#[derive(Debug, Parser)]
pub struct InterfaceDef {
    pub header: InterfaceHeader,
    pub interface_body: Option<InterfaceBody>,
}

#[derive(Debug, Parser)]
pub struct InterfaceHeader {
    pub kind: InterfaceKind,
    pub ident: Identifier,
    pub parent: Option<InterfaceInheritanceSpec>,
}

#[derive(Debug, Parser)]
pub struct InterfaceInheritanceSpec(pub Vec<InterfaceName>);

#[derive(Debug, Parser)]
pub struct InterfaceName(pub ScopedName);

#[derive(Debug, Parser)]
pub struct InterfaceBody(pub Vec<Export>);

#[derive(Debug, Parser)]
pub enum Export {
    OpDcl(OpDcl),
    AttrDcl(AttrDcl),
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    ExceptDcl(ExceptDcl),
}

#[derive(Debug, Parser)]
pub struct OpDcl {
    pub ty: OpTypeSpec,
    pub ident: Identifier,
    pub parameter: Option<ParameterDcls>,
    pub raises: Option<RaisesExpr>,
}
#[derive(Debug)]
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
                derive::node_id!("type_spec") => Ok(Self::TypeSpec(
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

#[derive(Debug, Parser)]
pub struct ParameterDcls(pub Vec<ParamDcl>);

#[derive(Debug, Parser)]
pub struct ParamDcl {
    pub attr: Option<ParamAttribute>,
    pub ty: TypeSpec,
    pub declarator: SimpleDeclarator,
}

#[derive(Debug)]
pub struct ParamAttribute(pub String);

impl<'a> crate::parser::FromTreeSitter<'a> for ParamAttribute {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), derive::node_id!("param_attribute"));

        Ok(Self(ctx.node_text(&node)?.to_string()))
    }
}

#[derive(Debug, Parser)]
pub struct RaisesExpr(pub Vec<ScopedName>);

#[derive(Debug, Parser)]
pub enum AttrDcl {
    ReadonlyAttrSpec(ReadonlyAttrSpec),
    AttrSpec(AttrSpec),
}

#[derive(Debug, Parser)]
pub struct ReadonlyAttrSpec {
    pub ty: TypeSpec,
    pub declarator: ReadonlyAttrDeclarator,
}

#[derive(Debug, Parser)]
pub enum ReadonlyAttrDeclarator {
    SimpleDeclarator(SimpleDeclarator),
    RaisesExpr(RaisesExpr),
}

#[derive(Debug, Parser)]
pub struct AttrSpec {
    pub type_spec: TypeSpec,
    pub declarator: AttrDeclarator,
}

#[derive(Debug)]
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
                derive::node_id!("simple_declarator") => {
                    declarator.push(SimpleDeclarator::from_node(ch, ctx)?);
                }
                derive::node_id!("attr_raises_expr") => {
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

#[derive(Debug)]
pub enum AttrRaisesExpr {
    Case1(GetExcepExpr, Option<SetExcepExpr>),
    SetExcepExpr(SetExcepExpr),
}

#[derive(Debug, Parser)]
pub struct GetExcepExpr {
    pub expr: ExceptionList,
}

#[derive(Debug, Parser)]
pub struct SetExcepExpr {
    pub expr: ExceptionList,
}

#[derive(Debug, Parser)]
pub struct ExceptionList(pub Vec<ScopedName>);

impl<'a> crate::parser::FromTreeSitter<'a> for AttrRaisesExpr {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), derive::node_id!("attr_raises_expr"));
        let mut get_excep = None;
        let mut set_excep = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                derive::node_id!("get_excep_expr") => {
                    get_excep = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                }
                derive::node_id!("set_excep_expr") => {
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
