mod enum_dcl;
pub use enum_dcl::*;

mod struct_dcl;
use serde::{Deserialize, Serialize};
pub use struct_dcl::*;

mod annotation;
pub use annotation::*;

mod expr;
pub use expr::*;

mod declarator;
pub use declarator::*;

mod types;
pub use types::*;

mod const_dcl;
pub use const_dcl::*;

mod interface;
pub use interface::*;

mod type_dcl;
pub use type_dcl::*;

mod exception_dcl;
pub use exception_dcl::*;

mod interface_codegen;
#[derive(Debug, Serialize, Deserialize)]
pub struct Specification(pub Vec<Definition>);

#[derive(Debug, Serialize, Deserialize)]
pub enum Definition {
    ConstrTypeDcl(ConstrTypeDcl),
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    ExceptDcl(ExceptDcl),
    InterfaceDcl(InterfaceDcl),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConstrTypeDcl {
    StructForwardDcl(StructForwardDcl),
    StructDcl(StructDcl),
    EnumDcl(EnumDcl),
    UnionForwardDcl(UnionForwardDcl),
    UnionDef(UnionDef),
    BitsetDcl(BitsetDcl),
    BitmaskDcl(BitmaskDcl),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnionForwardDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnionDef {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub switch_type_spec: SwitchTypeSpec,
    pub case: Vec<Case>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Case {
    pub label: Vec<CaseLabel>,
    pub element: ElementSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CaseLabel {
    Value(ConstExpr),
    Default,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementSpec {
    pub annotations: Vec<Annotation>,
    pub ty: ElementSpecTy,
    pub value: Declarator,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ElementSpecTy {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SwitchTypeSpec {
    IntegerType(IntegerType),
    CharType,
    WideCharType,
    BooleanType,
    ScopedName(ScopedName),
    OctetType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitsetDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub parent: Option<ScopedName>,
    pub field: Vec<BitField>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BitFieldType {
    Bool,
    Octec,
    SignedInt,
    UnsignedInt,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitField {
    pub ident: String,
    pub pos: PositiveIntConst,
    pub ty: Option<BitFieldType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitmaskDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub value: Vec<BitValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitValue {
    pub annotations: Vec<Annotation>,
    pub ident: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedName {
    pub name: Vec<String>,
    pub is_root: bool,
}

impl From<crate::typed_ast::ScopedName> for ScopedName {
    fn from(typed_ast: crate::typed_ast::ScopedName) -> Self {
        let is_root = typed_ast.node_text.starts_with("::");
        let mut v = vec![];
        get_scoped_name(&mut v, &typed_ast);
        let name = v.into_iter().map(ToOwned::to_owned).collect();

        Self { name, is_root }
    }
}

fn get_scoped_name<'a>(pre: &mut Vec<&'a str>, value: &'a crate::typed_ast::ScopedName) {
    if let Some(value) = &value.scoped_name {
        get_scoped_name(pre, value);
    }

    pre.push(&value.identifier.0);
}

impl From<crate::typed_ast::Specification> for Specification {
    fn from(value: crate::typed_ast::Specification) -> Self {
        spec_from_typed_ast(value, true)
    }
}

pub(crate) fn spec_from_typed_ast(
    value: crate::typed_ast::Specification,
    expand_interfaces: bool,
) -> Specification {
    let mut defs = Vec::new();
    let mut modules = Vec::new();
    collect_defs(value.0, &mut modules, expand_interfaces, &mut defs);
    Specification(defs)
}

fn collect_defs(
    defs: Vec<crate::typed_ast::Definition>,
    modules: &mut Vec<String>,
    expand_interfaces: bool,
    out: &mut Vec<Definition>,
) {
    for def in defs {
        match def {
            crate::typed_ast::Definition::ModuleDcl(module) => {
                modules.push(module.ident.0);
                collect_defs(module.definition, modules, expand_interfaces, out);
                modules.pop();
            }
            crate::typed_ast::Definition::TypeDcl(type_dcl) => {
                let type_dcl: TypeDcl = type_dcl.into();
                out.push(Definition::TypeDcl(type_dcl));
            }
            crate::typed_ast::Definition::ConstDcl(const_dcl) => {
                out.push(Definition::ConstDcl(const_dcl.into()));
            }
            crate::typed_ast::Definition::ExceptDcl(except_dcl) => {
                out.push(Definition::ExceptDcl(except_dcl.into()));
            }
            crate::typed_ast::Definition::InterfaceDcl(interface_dcl) => {
                let interface: InterfaceDcl = interface_dcl.into();
                if expand_interfaces {
                    let extra = interface_codegen::expand_interface(&interface, modules)
                        .unwrap_or_else(|err| {
                            panic!("interface expansion failed: {err}");
                        });
                    out.extend(extra);
                }
                out.push(Definition::InterfaceDcl(interface));
            }
            crate::typed_ast::Definition::TemplateModuleDcl(_)
            | crate::typed_ast::Definition::TemplateModuleInst(_)
            | crate::typed_ast::Definition::PreprocInclude(_)
            | crate::typed_ast::Definition::PreprocCall(_)
            | crate::typed_ast::Definition::PreprocDefine(_) => {}
        }
    }
}

fn apply_constr_annotations(constr: &mut ConstrTypeDcl, annotations: Vec<Annotation>) {
    match constr {
        ConstrTypeDcl::StructForwardDcl(def) => def.annotations = annotations,
        ConstrTypeDcl::StructDcl(def) => def.annotations = annotations,
        ConstrTypeDcl::EnumDcl(def) => def.annotations = annotations,
        ConstrTypeDcl::UnionForwardDcl(def) => def.annotations = annotations,
        ConstrTypeDcl::UnionDef(def) => def.annotations = annotations,
        ConstrTypeDcl::BitsetDcl(def) => def.annotations = annotations,
        ConstrTypeDcl::BitmaskDcl(def) => def.annotations = annotations,
    }
}

impl From<crate::typed_ast::ConstrTypeDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::ConstrTypeDcl) -> Self {
        match value {
            crate::typed_ast::ConstrTypeDcl::StructDcl(struct_dcl) => struct_dcl.into(),
            crate::typed_ast::ConstrTypeDcl::UnionDcl(union_dcl) => union_dcl.into(),
            crate::typed_ast::ConstrTypeDcl::EnumDcl(enum_dcl) => Self::EnumDcl(enum_dcl.into()),
            crate::typed_ast::ConstrTypeDcl::BitsetDcl(bitset_dcl) => {
                Self::BitsetDcl(bitset_dcl.into())
            }
            crate::typed_ast::ConstrTypeDcl::BitmaskDcl(bitmask_dcl) => {
                Self::BitmaskDcl(bitmask_dcl.into())
            }
        }
    }
}

impl From<crate::typed_ast::StructDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::StructDcl) -> Self {
        match value {
            crate::typed_ast::StructDcl::StructForwardDcl(forward) => {
                Self::StructForwardDcl(forward.into())
            }
            crate::typed_ast::StructDcl::StructDef(def) => Self::StructDcl(def.into()),
        }
    }
}

impl From<crate::typed_ast::UnionDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::UnionDcl) -> Self {
        match value {
            crate::typed_ast::UnionDcl::UnionForwardDcl(forward) => {
                Self::UnionForwardDcl(forward.into())
            }
            crate::typed_ast::UnionDcl::UnionDef(def) => Self::UnionDef(def.into()),
        }
    }
}

impl From<crate::typed_ast::UnionForwardDcl> for UnionForwardDcl {
    fn from(value: crate::typed_ast::UnionForwardDcl) -> Self {
        Self {
            annotations: vec![],
            ident: value.0 .0,
        }
    }
}

impl From<crate::typed_ast::UnionDef> for UnionDef {
    fn from(value: crate::typed_ast::UnionDef) -> Self {
        Self {
            annotations: vec![],
            ident: value.ident.0,
            switch_type_spec: value.switch_type_spec.into(),
            case: value.case.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::typed_ast::Case> for Case {
    fn from(value: crate::typed_ast::Case) -> Self {
        Self {
            label: value.label.into_iter().map(Into::into).collect(),
            element: value.element.into(),
        }
    }
}

impl From<crate::typed_ast::CaseLabel> for CaseLabel {
    fn from(value: crate::typed_ast::CaseLabel) -> Self {
        match value {
            crate::typed_ast::CaseLabel::Case(expr) => Self::Value(expr.into()),
            crate::typed_ast::CaseLabel::Default => Self::Default,
        }
    }
}

impl From<crate::typed_ast::ElementSpec> for ElementSpec {
    fn from(value: crate::typed_ast::ElementSpec) -> Self {
        Self {
            annotations: expand_annotations(value.annotations),
            ty: value.ty.into(),
            value: value.value.into(),
        }
    }
}

impl From<crate::typed_ast::ElementSpecTy> for ElementSpecTy {
    fn from(value: crate::typed_ast::ElementSpecTy) -> Self {
        match value {
            crate::typed_ast::ElementSpecTy::TypeSpec(ty) => Self::TypeSpec(ty.into()),
            crate::typed_ast::ElementSpecTy::ConstrTypeDcl(constr) => {
                Self::ConstrTypeDcl(constr.into())
            }
        }
    }
}

impl From<crate::typed_ast::SwitchTypeSpec> for SwitchTypeSpec {
    fn from(value: crate::typed_ast::SwitchTypeSpec) -> Self {
        match value {
            crate::typed_ast::SwitchTypeSpec::IntegerType(integer_type) => {
                Self::IntegerType(integer_type.into())
            }
            crate::typed_ast::SwitchTypeSpec::CharType(_) => Self::CharType,
            crate::typed_ast::SwitchTypeSpec::WideCharType(_) => Self::WideCharType,
            crate::typed_ast::SwitchTypeSpec::BooleanType(_) => Self::BooleanType,
            crate::typed_ast::SwitchTypeSpec::ScopedName(scoped_name) => {
                Self::ScopedName(scoped_name.into())
            }
            crate::typed_ast::SwitchTypeSpec::OctetType(_) => Self::OctetType,
        }
    }
}

impl From<crate::typed_ast::BitsetDcl> for BitsetDcl {
    fn from(value: crate::typed_ast::BitsetDcl) -> Self {
        let mut field = Vec::new();
        for bitfield in value.field {
            let pos = bitfield.spec.pos;
            let ty = bitfield.spec.dst_ty.map(Into::into);
            for ident in bitfield.ident {
                field.push(BitField {
                    ident: ident.0,
                    pos: pos.clone().into(),
                    ty: ty.clone(),
                });
            }
        }

        Self {
            annotations: vec![],
            ident: value.ident.0,
            parent: value.parent.map(Into::into),
            field,
        }
    }
}

impl From<crate::typed_ast::DestinationType> for BitFieldType {
    fn from(value: crate::typed_ast::DestinationType) -> Self {
        match value {
            crate::typed_ast::DestinationType::BooleanType(_) => Self::Bool,
            crate::typed_ast::DestinationType::OctetType(_) => Self::Octec,
            crate::typed_ast::DestinationType::IntegerType(integer_type) => {
                if matches!(integer_type, crate::typed_ast::IntegerType::SignedInt(_)) {
                    Self::SignedInt
                } else {
                    Self::UnsignedInt
                }
            }
        }
    }
}

impl From<crate::typed_ast::BitmaskDcl> for BitmaskDcl {
    fn from(value: crate::typed_ast::BitmaskDcl) -> Self {
        Self {
            annotations: vec![],
            ident: value.ident.0,
            value: value.value.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::typed_ast::BitValue> for BitValue {
    fn from(value: crate::typed_ast::BitValue) -> Self {
        Self {
            annotations: expand_annotations(value.annotations),
            ident: value.ident.0,
        }
    }
}
