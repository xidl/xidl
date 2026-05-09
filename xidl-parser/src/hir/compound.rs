use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConstrTypeDcl {
    StructForwardDcl(StructForwardDcl),
    StructDcl(StructDcl),
    EnumDcl(EnumDcl),
    UnionForwardDcl(UnionForwardDcl),
    UnionDef(UnionDef),
    BitsetDcl(BitsetDcl),
    BitmaskDcl(BitmaskDcl),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnionForwardDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnionDef {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub switch_type_spec: SwitchTypeSpec,
    pub case: Vec<Case>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Case {
    pub label: Vec<CaseLabel>,
    pub element: ElementSpec,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CaseLabel {
    Value(ConstExpr),
    Default,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ElementSpec {
    pub annotations: Vec<Annotation>,
    pub ty: ElementSpecTy,
    pub value: Declarator,
    pub field_id: Option<u32>,
    pub recursive: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ElementSpecTy {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SwitchTypeSpec {
    IntegerType(IntegerType),
    CharType,
    WideCharType,
    BooleanType,
    ScopedName(ScopedName),
    OctetType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BitField {
    pub ident: String,
    pub pos: PositiveIntConst,
    pub ty: Option<BitFieldType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BitmaskDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub value: Vec<BitValue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BitValue {
    pub position: usize,
    pub annotations: Vec<Annotation>,
    pub ident: String,
}

impl From<crate::typed_ast::ConstrTypeDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::ConstrTypeDcl) -> Self {
        match value {
            crate::typed_ast::ConstrTypeDcl::StructDcl(value) => value.into(),
            crate::typed_ast::ConstrTypeDcl::UnionDcl(value) => value.into(),
            crate::typed_ast::ConstrTypeDcl::EnumDcl(value) => Self::EnumDcl(value.into()),
            crate::typed_ast::ConstrTypeDcl::BitsetDcl(value) => Self::BitsetDcl(value.into()),
            crate::typed_ast::ConstrTypeDcl::BitmaskDcl(value) => Self::BitmaskDcl(value.into()),
        }
    }
}

impl From<crate::typed_ast::StructDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::StructDcl) -> Self {
        match value {
            crate::typed_ast::StructDcl::StructForwardDcl(value) => {
                Self::StructForwardDcl(value.into())
            }
            crate::typed_ast::StructDcl::StructDef(value) => Self::StructDcl(value.into()),
        }
    }
}

impl From<crate::typed_ast::UnionDcl> for ConstrTypeDcl {
    fn from(value: crate::typed_ast::UnionDcl) -> Self {
        match value {
            crate::typed_ast::UnionDcl::UnionForwardDcl(value) => {
                Self::UnionForwardDcl(value.into())
            }
            crate::typed_ast::UnionDcl::UnionDef(value) => Self::UnionDef(value.into()),
        }
    }
}

impl From<crate::typed_ast::UnionForwardDcl> for UnionForwardDcl {
    fn from(value: crate::typed_ast::UnionForwardDcl) -> Self {
        Self {
            annotations: vec![],
            ident: value.0.0,
        }
    }
}

impl From<crate::typed_ast::UnionDef> for UnionDef {
    fn from(value: crate::typed_ast::UnionDef) -> Self {
        let mut cases = value
            .case
            .into_iter()
            .map(Case::from)
            .collect::<Vec<Case>>();
        let mut member_ids = std::collections::HashMap::new();
        let mut next_field_id = 1u32;

        for case in &mut cases {
            let name = declarator_name(&case.element.value).to_string();
            if let Some(id) = case.element.field_id {
                let entry = member_ids.entry(name.clone()).or_insert(id);
                case.element.field_id = Some(*entry);
            } else if let Some(existing) = member_ids.get(&name) {
                case.element.field_id = Some(*existing);
            } else {
                member_ids.insert(name, next_field_id);
                case.element.field_id = Some(next_field_id);
                next_field_id += 1;
            }
        }

        Self {
            annotations: vec![],
            ident: value.ident.0,
            switch_type_spec: value.switch_type_spec.into(),
            case: cases,
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
            crate::typed_ast::CaseLabel::Case(value) => Self::Value(value.into()),
            crate::typed_ast::CaseLabel::Default => Self::Default,
        }
    }
}

impl From<crate::typed_ast::ElementSpec> for ElementSpec {
    fn from(value: crate::typed_ast::ElementSpec) -> Self {
        let annotations = expand_annotations(value.annotations);
        Self {
            field_id: annotation_id_value(&annotations),
            annotations,
            ty: value.ty.into(),
            value: value.value.into(),
            recursive: false,
        }
    }
}

impl From<crate::typed_ast::ElementSpecTy> for ElementSpecTy {
    fn from(value: crate::typed_ast::ElementSpecTy) -> Self {
        match value {
            crate::typed_ast::ElementSpecTy::TypeSpec(value) => Self::TypeSpec(value.into()),
            crate::typed_ast::ElementSpecTy::ConstrTypeDcl(value) => {
                Self::ConstrTypeDcl(value.into())
            }
        }
    }
}

impl From<crate::typed_ast::SwitchTypeSpec> for SwitchTypeSpec {
    fn from(value: crate::typed_ast::SwitchTypeSpec) -> Self {
        match value {
            crate::typed_ast::SwitchTypeSpec::IntegerType(value) => Self::IntegerType(value.into()),
            crate::typed_ast::SwitchTypeSpec::CharType(_) => Self::CharType,
            crate::typed_ast::SwitchTypeSpec::WideCharType(_) => Self::WideCharType,
            crate::typed_ast::SwitchTypeSpec::BooleanType(_) => Self::BooleanType,
            crate::typed_ast::SwitchTypeSpec::ScopedName(value) => Self::ScopedName(value.into()),
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
            crate::typed_ast::DestinationType::IntegerType(value) => {
                if matches!(value, crate::typed_ast::IntegerType::SignedInt(_)) {
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
        let mut bits = vec![];
        for (idx, bitvalue) in value.value.into_iter().enumerate() {
            let mut position = idx;
            for annotation in &bitvalue.annotations {
                let crate::typed_ast::AnnotationName::Builtin(name) = &annotation.name else {
                    continue;
                };
                if !name.eq_ignore_ascii_case("position") {
                    continue;
                }

                let Some(crate::typed_ast::AnnotationParams::Raw(v)) = &annotation.params else {
                    continue;
                };

                if let Ok(v) = v.parse::<usize>() {
                    position = v;
                }
            }
            let mut bit: BitValue = bitvalue.into();
            bit.position = position;
            bits.push(bit);
        }
        Self {
            annotations: vec![],
            ident: value.ident.0,
            value: bits,
        }
    }
}

impl From<crate::typed_ast::BitValue> for BitValue {
    fn from(value: crate::typed_ast::BitValue) -> Self {
        Self {
            position: 0,
            annotations: expand_annotations(value.annotations),
            ident: value.ident.0,
        }
    }
}

fn declarator_name(value: &Declarator) -> &str {
    match value {
        Declarator::SimpleDeclarator(value) => &value.0,
        Declarator::ArrayDeclarator(value) => &value.ident,
    }
}
