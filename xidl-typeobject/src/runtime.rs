use crate::DDS::XTypes as xt;
use crate::XidlTypeObject;

#[derive(Clone, Copy, Debug)]
pub enum TypeEquivalence {
    Minimal,
    Complete,
}

pub struct StructMemberDesc<'a> {
    pub member_id: u32,
    pub member_flags: u32,
    pub type_id: xt::TypeIdentifier,
    pub name: &'a str,
}

pub struct UnionMemberDesc<'a> {
    pub member_id: u32,
    pub member_flags: u32,
    pub type_id: xt::TypeIdentifier,
    pub name: &'a str,
    pub labels: Vec<i32>,
}

pub struct EnumLiteralDesc<'a> {
    pub value: i32,
    pub flags: u32,
    pub name: &'a str,
}

pub struct BitflagDesc<'a> {
    pub position: u16,
    pub flags: u32,
    pub name: &'a str,
}

pub struct BitfieldDesc<'a> {
    pub position: u16,
    pub flags: u32,
    pub bitcount: u8,
    pub holder_type: xt::TypeKind,
    pub name: Option<&'a str>,
}

pub fn type_identifier_none() -> xt::TypeIdentifier {
    xt::TypeIdentifier::default()
}

pub fn type_identifier_primitive(_kind: xt::TypeKind) -> xt::TypeIdentifier {
    xt::TypeIdentifier::default()
}

pub fn type_identifier_sequence(
    _element_type: xt::TypeIdentifier,
    _bound: u32,
    _eq: TypeEquivalence,
) -> xt::TypeIdentifier {
    xt::TypeIdentifier::default()
}

pub fn type_identifier_array(
    _element_type: xt::TypeIdentifier,
    _dims: Vec<u32>,
    _eq: TypeEquivalence,
) -> xt::TypeIdentifier {
    xt::TypeIdentifier::default()
}

pub fn type_identifier_map(
    _key_type: xt::TypeIdentifier,
    _value_type: xt::TypeIdentifier,
    _bound: u32,
    _eq: TypeEquivalence,
) -> xt::TypeIdentifier {
    xt::TypeIdentifier::default()
}

pub fn type_identifier_for<T: XidlTypeObject + 'static>(
    _eq: TypeEquivalence,
) -> xt::TypeIdentifier {
    xt::TypeIdentifier::default()
}

pub fn build_complete_alias(
    _name: &str,
    _type_flags: u32,
    _related_type: xt::TypeIdentifier,
) -> xt::TypeObject {
    xt::TypeObject::new_complete(xt::CompleteTypeObject::default())
}

pub fn build_minimal_alias(_type_flags: u32, _related_type: xt::TypeIdentifier) -> xt::TypeObject {
    xt::TypeObject::new_minimal(xt::MinimalTypeObject::default())
}

pub fn build_complete_struct(
    _name: &str,
    _type_flags: u32,
    _header_type_id: xt::TypeIdentifier,
    _members: Vec<StructMemberDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_complete(xt::CompleteTypeObject::default())
}

pub fn build_minimal_struct(
    _type_flags: u32,
    _header_type_id: xt::TypeIdentifier,
    _members: Vec<StructMemberDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_minimal(xt::MinimalTypeObject::default())
}

pub fn build_complete_union(
    _name: &str,
    _type_flags: u32,
    _header_type_id: xt::TypeIdentifier,
    _discriminator_type_id: xt::TypeIdentifier,
    _members: Vec<UnionMemberDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_complete(xt::CompleteTypeObject::default())
}

pub fn build_minimal_union(
    _type_flags: u32,
    _header_type_id: xt::TypeIdentifier,
    _discriminator_type_id: xt::TypeIdentifier,
    _members: Vec<UnionMemberDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_minimal(xt::MinimalTypeObject::default())
}

pub fn build_complete_enum(
    _name: &str,
    _type_flags: u32,
    _bit_bound: u16,
    _literals: Vec<EnumLiteralDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_complete(xt::CompleteTypeObject::default())
}

pub fn build_minimal_enum(
    _type_flags: u32,
    _bit_bound: u16,
    _literals: Vec<EnumLiteralDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_minimal(xt::MinimalTypeObject::default())
}

pub fn build_complete_bitmask(
    _name: &str,
    _type_flags: u32,
    _bit_bound: u16,
    _flags: Vec<BitflagDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_complete(xt::CompleteTypeObject::default())
}

pub fn build_minimal_bitmask(
    _type_flags: u32,
    _bit_bound: u16,
    _flags: Vec<BitflagDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_minimal(xt::MinimalTypeObject::default())
}

pub fn build_complete_bitset(
    _name: &str,
    _type_flags: u32,
    _fields: Vec<BitfieldDesc<'_>>,
) -> xt::TypeObject {
    xt::TypeObject::new_complete(xt::CompleteTypeObject::default())
}

pub fn build_minimal_bitset(_type_flags: u32, _fields: Vec<BitfieldDesc<'_>>) -> xt::TypeObject {
    xt::TypeObject::new_minimal(xt::MinimalTypeObject::default())
}
