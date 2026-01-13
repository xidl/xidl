//! ```js
//! exports.rules = {
//!   typedef_dcl: $ => seq('typedef', $.type_declarator),
//!   type_declarator: $ =>
//!     seq(
//!       choice($.simple_type_spec, $.template_type_spec, $.constr_type_dcl),
//!       $.any_declarators,
//!     ),
//!
//!   any_declarators: $ => commaSep1($.any_declarator),
//!   any_declarator: $ => choice($.simple_declarator, $.array_declarator),
//!   simple_declarator: $ => $.identifier,
//!   declarator: $ =>
//!     choice(
//!       $.simple_declarator,
//!       $.array_declarator, // 7.4.14
//!     ),
//!   declarators: $ => commaSep1($.declarator),
//!   array_declarator: $ => seq($.identifier, repeat1($.fixed_array_size)),
//!   fixed_array_size: $ => seq('[', $.positive_int_const, ']'),
//! }
//! ```

use super::*;
use xidl_derive::Parser;

#[derive(Debug, Parser)]
pub struct TypedefDcl {
    pub decl: TypeDeclarator,
}

#[derive(Debug)]
pub struct TypeDeclarator {
    pub ty: TypeDeclaratorInner,
    pub decl: AnyDeclarators,
}

#[derive(Debug)]
pub enum TypeDeclaratorInner {
    SimpleTypeSpec(SimpleTypeSpec),
    TemplateTypeSpec(TemplateTypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug, Parser)]
pub struct AnyDeclarators(pub Vec<AnyDeclarator>);

#[derive(Debug, Parser)]
pub enum AnyDeclarator {
    SimpleDeclarator(SimpleDeclarator),
    ArrayDeclarator(ArrayDeclarator),
}

#[derive(Debug, Parser)]
pub struct SimpleDeclarator(pub Identifier);

#[derive(Debug, Parser)]
pub enum Declarator {
    SimpleDeclarator(SimpleDeclarator),
    ArrayDeclarator(ArrayDeclarator),
}

#[derive(Debug, Parser)]
pub struct Declarators(pub Vec<Declarator>);

#[derive(Debug, Parser)]
pub struct ArrayDeclarator {
    pub ident: Identifier,
    pub len: Vec<FixedArraySize>,
}

#[derive(Debug, Parser)]
pub struct FixedArraySize(pub PositiveIntConst);

impl<'a> crate::parser::FromTreeSitter<'a> for TypeDeclarator {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_derive::node_id!("type_declarator"));
        let mut ty = None;
        let mut decl = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_derive::node_id!("simple_type_spec")
                | xidl_derive::node_id!("template_type_spec")
                | xidl_derive::node_id!("constr_type_dcl") => {
                    ty = Some(TypeDeclaratorInner::from_node(ch, ctx)?);
                }
                xidl_derive::node_id!("any_declarators") => {
                    decl = Some(AnyDeclarators::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            ty: ty.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing type",
                    node.kind()
                ))
            })?,
            decl: decl.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing declarators",
                    node.kind()
                ))
            })?,
        })
    }
}

impl<'a> crate::parser::FromTreeSitter<'a> for TypeDeclaratorInner {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        match node.kind_id() {
            xidl_derive::node_id!("simple_type_spec") => Ok(Self::SimpleTypeSpec(
                crate::parser::FromTreeSitter::from_node(node, ctx)?,
            )),
            xidl_derive::node_id!("template_type_spec") => Ok(Self::TemplateTypeSpec(
                crate::parser::FromTreeSitter::from_node(node, ctx)?,
            )),
            xidl_derive::node_id!("constr_type_dcl") => Ok(Self::ConstrTypeDcl(
                crate::parser::FromTreeSitter::from_node(node, ctx)?,
            )),
            _ => Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: type_declarator, got: {}",
                node.kind()
            ))),
        }
    }
}
