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

#[derive(Debug)]
pub enum UnionDcl {
    UnionDef(UnionDef),
    UnionForwardDcl(UnionForwardDcl),
}

#[derive(Debug)]
pub struct UnionDef {
    pub ident: Identifier,
    pub switch_type_spec: SwitchTypeSpec,
    pub case: Vec<Case>,
}
#[derive(Debug)]
pub struct UnionForwardDcl(pub Identifier);

#[derive(Debug)]
pub struct Case {
    pub label: Vec<CaseLabel>,
    pub element: ElementSpec,
}

#[derive(Debug)]
pub struct CaseLabel(pub ConstExpr);

#[derive(Debug)]
pub struct ElementSpec {
    pub ty: ElementSpecTy,
    pub value: Declarator,
}

#[derive(Debug)]
pub enum ElementSpecTy {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug)]
pub enum SwitchTypeSpec {
    IntegerType(IntegerType),
    CharType(CharType),
    WideCharType(WideCharType),
    BooleanType(BooleanType),
    ScopedName(ScopedName),
    OctetType(OctetType),
}
