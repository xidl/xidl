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

pub struct TypedefDcl {
    pub decl: TypeDeclarator,
}

pub struct TypeDeclarator {
    pub ty: TypeDeclaratorInner,
    pub decl: AnyDeclarators,
}

pub enum TypeDeclaratorInner {
    SimpleTypeSpec(SimpleTypeSpec),
    TemplateTypeSpec(TemplateTypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug)]
pub struct AnyDeclarators(pub Vec<AnyDeclarator>);

#[derive(Debug)]
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
