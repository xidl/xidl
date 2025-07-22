use super::*;

pub struct EnumDcl {
    pub ident: Identifier,
    pub member: Vec<Enumerator>,
}

pub struct Enumerator {
    pub ident: Identifier,
}

pub enum UnionDcl {
    UnionDef(UnionDef),
    UnionForwardDcl(UnionForwardDcl),
}

pub struct UnionDef {
    pub ident: Identifier,
    pub switch_type_spec: SwitchTypeSpec,
    pub case: Vec<Case>,
}
pub struct UnionForwardDcl(pub Identifier);

pub struct Case {
    pub label: Vec<CaseLabel>,
    pub element: ElementSpec,
}

pub struct CaseLabel(pub ConstExpr);

pub struct ElementSpec {
    pub ty: ElementSpecTy,
    pub value: Declarator,
}

pub enum ElementSpecTy {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

pub enum SwitchTypeSpec {
    IntegerType(IntegerType),
    CharType(CharType),
    WideCharType(WideCharType),
    BooleanType(BooleanType),
    ScopedName(ScopedName),
    OctetType(OctetType),
}
