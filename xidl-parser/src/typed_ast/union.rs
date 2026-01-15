use super::*;

#[derive(Debug, Parser)]
pub struct EnumDcl {
    pub ident: Identifier,
    pub member: Vec<Enumerator>,
}

#[derive(Debug, Parser)]
pub struct Enumerator {
    pub annotations: Vec<AnnotationAppl>,
    pub ident: Identifier,
}

#[derive(Debug, Parser)]
pub enum UnionDcl {
    UnionDef(UnionDef),
    UnionForwardDcl(UnionForwardDcl),
}

#[derive(Debug, Parser)]
pub struct UnionDef {
    pub ident: Identifier,
    pub switch_type_spec: SwitchTypeSpec,
    pub case: Vec<Case>,
}
#[derive(Debug, Parser)]
pub struct UnionForwardDcl(pub Identifier);

#[derive(Debug, Parser)]
pub struct Case {
    pub label: Vec<CaseLabel>,
    pub element: ElementSpec,
}

#[derive(Debug)]
pub enum CaseLabel {
    Case(ConstExpr),
    Default,
}

impl<'a> crate::parser::FromTreeSitter<'a> for CaseLabel {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_derive::node_id!("case_label"));
        for ch in node.children(&mut node.walk()) {
            if ch.kind_id() == xidl_derive::node_id!("const_expr") {
                return Ok(Self::Case(ConstExpr::from_node(ch, ctx)?));
            }
        }

        let text = ctx.node_text(&node)?.trim();
        if text.starts_with("default") {
            return Ok(Self::Default);
        }

        Err(crate::error::ParseError::UnexpectedNode(format!(
            "parent: {}, got: missing case label",
            node.kind()
        )))
    }
}

#[derive(Debug)]
pub struct ElementSpec {
    pub annotations: Vec<AnnotationAppl>,
    pub ty: ElementSpecTy,
    pub value: Declarator,
}

impl<'a> crate::parser::FromTreeSitter<'a> for ElementSpec {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_derive::node_id!("element_spec"));
        let mut annotations = vec![];
        let mut ty = None;
        let mut value = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_derive::node_id!("annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_derive::node_id!("type_spec") | xidl_derive::node_id!("constr_type_dcl") => {
                    ty = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                }
                xidl_derive::node_id!("declarator") => {
                    value = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            annotations,
            ty: ty.unwrap(),
            value: value.unwrap(),
        })
    }
}

#[derive(Debug, Parser)]
#[ts(transparent)]
pub enum ElementSpecTy {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug, Parser)]
pub enum SwitchTypeSpec {
    IntegerType(IntegerType),
    CharType(CharType),
    WideCharType(WideCharType),
    BooleanType(BooleanType),
    ScopedName(ScopedName),
    OctetType(OctetType),
}
