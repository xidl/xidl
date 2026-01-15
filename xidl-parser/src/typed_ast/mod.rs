mod base_types;
pub use base_types::*;

mod expr;
pub use expr::*;
use xidl_derive::Parser;

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

#[derive(Debug, Parser)]
pub struct Specification(pub Vec<Definition>);

#[derive(Debug, Parser)]
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

#[derive(Debug)]
pub struct TypeDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub decl: TypeDclInner,
}

#[derive(Debug, Parser)]
#[ts(transparent)]
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
        assert_eq!(node.kind_id(), xidl_derive::node_id!("type_dcl"));
        let mut annotations = Vec::new();
        let mut decl = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_derive::node_id!("annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_derive::node_id!("constr_type_dcl")
                | xidl_derive::node_id!("native_dcl")
                | xidl_derive::node_id!("typedef_dcl") => {
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

#[derive(Debug, Parser)]
pub struct NativeDcl {
    pub decl: SimpleDeclarator,
}

#[derive(Debug, Parser)]
pub enum ConstrTypeDcl {
    StructDcl(StructDcl),
    UnionDcl(UnionDcl),
    EnumDcl(EnumDcl),
    BitsetDcl(BitsetDcl),
    BitmaskDcl(BitmaskDcl),
}

#[derive(Debug, Parser)]
pub enum StructDcl {
    StructForwardDcl(StructForwardDcl),
    StructDef(StructDef),
}

#[derive(Debug, Parser)]
pub struct StructForwardDcl {
    pub ident: Identifier,
}

#[derive(Debug, Parser)]
pub struct StructDef {
    pub ident: Identifier,
    pub parent: Vec<ScopedName>,
    pub member: Vec<Member>,
}

#[derive(Debug, Parser)]
pub struct Member {
    pub annotations: Vec<AnnotationAppl>,
    pub ty: TypeSpec,
    pub ident: Declarators,
    pub default: Option<Default>,
}

#[derive(Debug, Parser)]
pub struct Default(pub ConstExpr);

#[derive(Debug, Parser)]
pub struct ConstDcl {
    pub ty: ConstType,
    pub ident: Identifier,
    pub value: ConstExpr,
}

#[derive(Debug, Parser)]
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

#[derive(Debug, Clone, PartialEq, Parser)]
#[ts(transparent)]
pub struct Identifier(pub String);

#[derive(Debug, Clone, Parser)]
pub struct PositiveIntConst(pub ConstExpr);
