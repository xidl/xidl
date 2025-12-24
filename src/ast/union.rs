use super::*;

#[derive(Debug, Parser)]
pub struct EnumDcl {
    pub ident: Identifier,
    pub member: Vec<Enumerator>,
}

#[derive(Debug, Parser)]
pub struct Enumerator {
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

#[derive(Debug, Parser)]
pub struct CaseLabel(pub ConstExpr);

#[derive(Debug, Parser)]
pub struct ElementSpec {
    pub ty: ElementSpecTy,
    pub value: Declarator,
}

#[derive(Debug, Parser)]
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
