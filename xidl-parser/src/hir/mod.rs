mod enum_dcl;
pub use enum_dcl::*;

mod struct_dcl;
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

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Specification(pub Vec<Definition>);

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ParserProperties {
    pub expand_interface: bool,
}

impl std::default::Default for ParserProperties {
    fn default() -> Self {
        Self {
            expand_interface: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Definition {
    ModuleDcl(ModuleDcl),
    Pragma(Pragma),
    ConstrTypeDcl(ConstrTypeDcl),
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    ExceptDcl(ExceptDcl),
    InterfaceDcl(InterfaceDcl),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub definition: Vec<Definition>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SerializeKind {
    Cdr,
    PlainCdr,
    PlCdr,
    PlainCdr2,
    DelimitedCdr,
    PlCdr2,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SerializeVersion {
    Xcdr1,
    Xcdr2,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct SerializeConfig {
    pub explicit_kind: Option<SerializeKind>,
    pub version: Option<SerializeVersion>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Pragma {
    XidlcSerialize(SerializeKind),
    XidlcVersion(SerializeVersion),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Extensibility {
    Final,
    Appendable,
    Mutable,
    None,
}

impl SerializeConfig {
    pub fn apply_pragma(&mut self, pragma: Pragma) {
        match pragma {
            Pragma::XidlcSerialize(kind) => {
                self.explicit_kind = Some(kind);
            }
            Pragma::XidlcVersion(version) => {
                self.version = Some(version);
                self.explicit_kind = None;
            }
        }
    }

    pub fn resolve(&self, extensibility: Extensibility) -> SerializeKind {
        if let Some(kind) = self.explicit_kind {
            return kind;
        }

        match self.version {
            None => SerializeKind::Cdr,
            Some(SerializeVersion::Xcdr1) => match extensibility {
                Extensibility::Mutable => SerializeKind::PlCdr,
                Extensibility::Final | Extensibility::Appendable => SerializeKind::Cdr,
                Extensibility::None => SerializeKind::PlainCdr,
            },
            Some(SerializeVersion::Xcdr2) => match extensibility {
                Extensibility::Final => SerializeKind::PlainCdr2,
                Extensibility::Appendable => SerializeKind::DelimitedCdr,
                Extensibility::Mutable => SerializeKind::PlCdr2,
                Extensibility::None => SerializeKind::Cdr,
            },
        }
    }

    pub fn resolve_for_annotations(&self, annotations: &[Annotation]) -> SerializeKind {
        self.resolve(extensibility_from_annotations(annotations))
    }
}

pub fn extensibility_from_annotations(annotations: &[Annotation]) -> Extensibility {
    let mut final_flag = false;
    let mut appendable = false;
    let mut mutable = false;
    for anno in annotations {
        if let Annotation::Builtin { name, .. } = anno {
            if name.eq_ignore_ascii_case("final") {
                final_flag = true;
            } else if name.eq_ignore_ascii_case("appendable") {
                appendable = true;
            } else if name.eq_ignore_ascii_case("mutable") {
                mutable = true;
            }
        }
        if let Annotation::Builtin { name, params } = anno {
            if name.eq_ignore_ascii_case("extensibility") {
                if let Some(AnnotationParams::Raw(raw)) = params {
                    let value = raw.trim().trim_matches('"');
                    if value.eq_ignore_ascii_case("final") {
                        final_flag = true;
                    } else if value.eq_ignore_ascii_case("appendable") {
                        appendable = true;
                    } else if value.eq_ignore_ascii_case("mutable") {
                        mutable = true;
                    }
                }
            }
        }
    }

    if mutable {
        Extensibility::Mutable
    } else if appendable {
        Extensibility::Appendable
    } else if final_flag {
        Extensibility::Final
    } else {
        Extensibility::None
    }
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

impl UnionDef {
    pub fn serialize_kind(&self, config: &SerializeConfig) -> SerializeKind {
        config.resolve_for_annotations(&self.annotations)
    }
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
    pub field_id: Option<u32>,
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

impl BitsetDcl {
    pub fn serialize_kind(&self, config: &SerializeConfig) -> SerializeKind {
        config.resolve_for_annotations(&self.annotations)
    }
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

impl BitmaskDcl {
    pub fn serialize_kind(&self, config: &SerializeConfig) -> SerializeKind {
        config.resolve_for_annotations(&self.annotations)
    }
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

impl Specification {
    pub fn from_typed_ast_with_properties(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
    ) -> Self {
        spec_from_typed_ast(value, properties.expand_interface)
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
                let ident = module.ident.0;
                let annotations = expand_annotations(module.annotations);
                modules.push(ident.clone());
                let mut inner = Vec::new();
                collect_defs(module.definition, modules, expand_interfaces, &mut inner);
                modules.pop();
                out.push(Definition::ModuleDcl(ModuleDcl {
                    annotations,
                    ident,
                    definition: inner,
                }));
            }
            crate::typed_ast::Definition::PreprocCall(call) => {
                if let Some(pragma) = parse_xidlc_pragma(&call) {
                    out.push(Definition::Pragma(pragma));
                }
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
            | crate::typed_ast::Definition::PreprocDefine(_) => {}
        }
    }
}

fn parse_xidlc_pragma(call: &crate::typed_ast::PreprocCall) -> Option<Pragma> {
    let directive = call.directive.0.as_str();
    if !directive.eq_ignore_ascii_case("#pragma") && !directive.eq_ignore_ascii_case("#progma") {
        return None;
    }
    let arg = call.argument.as_ref()?.0.as_str();
    let mut parts = arg.split_whitespace();
    let namespace = parts.next()?;
    if !namespace.eq_ignore_ascii_case("xidlc") {
        return None;
    }
    let token = parts.next()?;

    if token.eq_ignore_ascii_case("XCDR1") {
        return Some(Pragma::XidlcVersion(SerializeVersion::Xcdr1));
    }
    if token.eq_ignore_ascii_case("XCDR2") {
        return Some(Pragma::XidlcVersion(SerializeVersion::Xcdr2));
    }

    if let Some(inner) = token
        .strip_prefix("serialize(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let inner = inner.trim();
        if inner.eq_ignore_ascii_case("XCDR1") {
            return Some(Pragma::XidlcVersion(SerializeVersion::Xcdr1));
        }
        if inner.eq_ignore_ascii_case("XCDR2") {
            return Some(Pragma::XidlcVersion(SerializeVersion::Xcdr2));
        }
        if let Some(kind) = parse_serialize_kind(inner) {
            return Some(Pragma::XidlcSerialize(kind));
        }
    }

    None
}

fn parse_serialize_kind(value: &str) -> Option<SerializeKind> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("CDR") {
        Some(SerializeKind::Cdr)
    } else if value.eq_ignore_ascii_case("PLAIN_CDR") {
        Some(SerializeKind::PlainCdr)
    } else if value.eq_ignore_ascii_case("PL_CDR") {
        Some(SerializeKind::PlCdr)
    } else if value.eq_ignore_ascii_case("PLAIN_CDR2") {
        Some(SerializeKind::PlainCdr2)
    } else if value.eq_ignore_ascii_case("DELIMITED_CDR") {
        Some(SerializeKind::DelimitedCdr)
    } else if value.eq_ignore_ascii_case("PL_CDR2") {
        Some(SerializeKind::PlCdr2)
    } else {
        None
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
        let mut cases = value
            .case
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Case>>();
        let mut member_ids = std::collections::HashMap::new();
        let mut next_field_id = 1u32;
        for case in cases.iter_mut() {
            let name = declarator_name(&case.element.value).to_string();
            if let Some(id) = case.element.field_id {
                let entry = member_ids.entry(name.clone()).or_insert(id);
                case.element.field_id = Some(*entry);
                continue;
            }
            if let Some(existing) = member_ids.get(&name) {
                case.element.field_id = Some(*existing);
                continue;
            }
            member_ids.insert(name, next_field_id);
            case.element.field_id = Some(next_field_id);
            next_field_id += 1;
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
            crate::typed_ast::CaseLabel::Case(expr) => Self::Value(expr.into()),
            crate::typed_ast::CaseLabel::Default => Self::Default,
        }
    }
}

impl From<crate::typed_ast::ElementSpec> for ElementSpec {
    fn from(value: crate::typed_ast::ElementSpec) -> Self {
        let annotations = expand_annotations(value.annotations);
        let field_id = annotation_id_value(&annotations);
        Self {
            annotations,
            ty: value.ty.into(),
            value: value.value.into(),
            field_id,
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

fn declarator_name(value: &Declarator) -> &str {
    match value {
        Declarator::SimpleDeclarator(value) => &value.0,
        Declarator::ArrayDeclarator(value) => &value.ident,
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
