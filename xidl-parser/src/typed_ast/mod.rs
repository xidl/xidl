mod base_types;
pub use base_types::*;
use serde::{Deserialize, Serialize};

mod expr;
pub use expr::*;
use xidl_parser_derive::Parser;

mod annotation;
pub use annotation::*;

mod preproc;
pub use preproc::*;

mod bitmask;
pub use bitmask::*;

mod interface;
pub use interface::*;

mod union;
pub use union::*;

mod typedef_dcl_imp;
pub use typedef_dcl_imp::*;

mod module_dcl;
pub use module_dcl::*;

mod exception_dcl;
pub use exception_dcl::*;

mod template_module;
pub use template_module::*;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct Specification(pub Vec<Definition>);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum Definition {
    ModuleDcl(ModuleDcl),
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    ExceptDcl(ExceptDcl),
    InterfaceDcl(InterfaceDcl),
    TemplateModuleDcl(TemplateModuleDcl),
    TemplateModuleInst(TemplateModuleInst),
    PreprocInclude(PreprocInclude),
    PreprocCall(PreprocCall),
    PreprocDefine(PreprocDefine),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub decl: TypeDclInner,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(transparent)]
#[allow(clippy::large_enum_variant)]
pub enum TypeDclInner {
    ConstrTypeDcl(ConstrTypeDcl),
    NativeDcl(NativeDcl),
    TypedefDcl(TypedefDcl),
}

impl<'a> crate::parser::FromTreeSitter<'a> for TypeDcl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("type_dcl"));
        let mut annotations = Vec::new();
        let mut decl = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("constr_type_dcl")
                | xidl_parser_derive::node_id!("native_dcl")
                | xidl_parser_derive::node_id!("typedef_dcl") => {
                    decl = Some(TypeDclInner::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            annotations,
            decl: decl.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing type decl",
                    node.kind()
                ))
            })?,
        })
    }
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct NativeDcl {
    pub decl: SimpleDeclarator,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum ConstrTypeDcl {
    StructDcl(StructDcl),
    UnionDcl(UnionDcl),
    EnumDcl(EnumDcl),
    BitsetDcl(BitsetDcl),
    BitmaskDcl(BitmaskDcl),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum StructDcl {
    StructForwardDcl(StructForwardDcl),
    StructDef(StructDef),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct StructForwardDcl {
    pub ident: Identifier,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct StructDef {
    pub ident: Identifier,
    pub parent: Vec<ScopedName>,
    pub member: Vec<Member>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Member {
    pub annotations: Vec<AnnotationAppl>,
    pub ty: TypeSpec,
    pub ident: Declarators,
    pub default: Option<Default>,
}

impl<'a> crate::parser::FromTreeSitter<'a> for Member {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("member"));
        let mut annotations = Vec::new();
        let mut ty = None;
        let mut ident = None;
        let mut default = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("type_spec") => {
                    ty = Some(TypeSpec::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("declarators") => {
                    ident = Some(Declarators::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("default") => {
                    default = Some(Default::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            annotations,
            ty: ty.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing type",
                    node.kind()
                ))
            })?,
            ident: ident.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing declarators",
                    node.kind()
                ))
            })?,
            default,
        })
    }
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct Default(pub ConstExpr);

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ConstDcl {
    pub ty: ConstType,
    pub ident: Identifier,
    pub value: ConstExpr,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Parser, Serialize, Deserialize)]
#[ts(transparent)]
pub struct Identifier(pub String);

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct PositiveIntConst(pub ConstExpr);
