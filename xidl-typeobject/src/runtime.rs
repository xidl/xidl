use crate::DDS::XTypes as xt;
use crate::XidlTypeObject;
use std::any::TypeId;
use std::cell::RefCell;
use xidl_xcdr::xcdr2::Xcdr2Serialize;

thread_local! {
    static TYPE_STACK: RefCell<Vec<TypeId>> = const{ RefCell::new(Vec::new()) };
}

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

pub fn name_hash(name: &str) -> [u8; 4] {
    let digest = md5::compute(name.as_bytes());
    let mut out = [0u8; 4];
    out.copy_from_slice(&digest.0[..4]);
    out
}

pub fn type_identifier_for<T: XidlTypeObject + 'static>(eq: TypeEquivalence) -> xt::TypeIdentifier {
    let type_id = TypeId::of::<T>();
    let reentrant = TYPE_STACK.with(|stack| stack.borrow().contains(&type_id));
    if reentrant {
        return type_identifier_none();
    }
    TYPE_STACK.with(|stack| stack.borrow_mut().push(type_id));
    let object = match eq {
        TypeEquivalence::Minimal => T::minimal_type_object(),
        TypeEquivalence::Complete => T::complete_type_object(),
    };
    let hash = match eq {
        TypeEquivalence::Minimal => {
            let value = object
                .minimal
                .as_ref()
                .expect("minimal type object missing");
            hash_minimal_type_object(value.as_ref())
        }
        TypeEquivalence::Complete => {
            let value = object
                .complete
                .as_ref()
                .expect("complete type object missing");
            hash_complete_type_object(value.as_ref())
        }
    };
    let out = type_identifier_hash(
        match eq {
            TypeEquivalence::Minimal => xt::EK_MINIMAL,
            TypeEquivalence::Complete => xt::EK_COMPLETE,
        },
        hash,
    );
    TYPE_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        let _ = stack.pop();
    });
    out
}

pub fn type_identifier_none() -> xt::TypeIdentifier {
    xt::TypeIdentifier {
        _d: xt::TK_NONE,
        string_sdefn: None,
        string_ldefn: None,
        seq_sdefn: None,
        seq_ldefn: None,
        array_sdefn: None,
        array_ldefn: None,
        map_sdefn: None,
        map_ldefn: None,
        sc_component_id: None,
        equivalence_hash: None,
        extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
    }
}

pub fn type_identifier_primitive(kind: xt::TypeKind) -> xt::TypeIdentifier {
    xt::TypeIdentifier {
        _d: kind,
        string_sdefn: None,
        string_ldefn: None,
        seq_sdefn: None,
        seq_ldefn: None,
        array_sdefn: None,
        array_ldefn: None,
        map_sdefn: None,
        map_ldefn: None,
        sc_component_id: None,
        equivalence_hash: None,
        extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
    }
}

pub fn type_identifier_string(bound: Option<u32>, wide: bool) -> xt::TypeIdentifier {
    let bound = bound.unwrap_or(0);
    let (small, large) = if wide {
        (xt::TI_STRING16_SMALL, xt::TI_STRING16_LARGE)
    } else {
        (xt::TI_STRING8_SMALL, xt::TI_STRING8_LARGE)
    };
    if bound <= u8::MAX as u32 {
        xt::TypeIdentifier {
            _d: small,
            string_sdefn: Some(Box::new(xt::StringSTypeDefn {
                bound: Box::new(bound as u8),
            })),
            string_ldefn: None,
            seq_sdefn: None,
            seq_ldefn: None,
            array_sdefn: None,
            array_ldefn: None,
            map_sdefn: None,
            map_ldefn: None,
            sc_component_id: None,
            equivalence_hash: None,
            extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
        }
    } else {
        xt::TypeIdentifier {
            _d: large,
            string_sdefn: None,
            string_ldefn: Some(Box::new(xt::StringLTypeDefn {
                bound: Box::new(bound),
            })),
            seq_sdefn: None,
            seq_ldefn: None,
            array_sdefn: None,
            array_ldefn: None,
            map_sdefn: None,
            map_ldefn: None,
            sc_component_id: None,
            equivalence_hash: None,
            extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
        }
    }
}

pub fn type_identifier_sequence(
    element_id: xt::TypeIdentifier,
    bound: u32,
    eq: TypeEquivalence,
) -> xt::TypeIdentifier {
    let equiv_kind = if is_fully_descriptive(&element_id) {
        xt::EK_BOTH
    } else {
        match eq {
            TypeEquivalence::Complete => xt::EK_COMPLETE,
            TypeEquivalence::Minimal => xt::EK_MINIMAL,
        }
    };
    if bound <= u8::MAX as u32 {
        xt::TypeIdentifier {
            _d: xt::TI_PLAIN_SEQUENCE_SMALL,
            string_sdefn: None,
            string_ldefn: None,
            seq_sdefn: Some(Box::new(xt::PlainSequenceSElemDefn {
                header: Box::new(xt::PlainCollectionHeader {
                    equiv_kind: Box::new(equiv_kind),
                    element_flags: Box::new(xt::CollectionElementFlag { value: 0 }),
                }),
                bound: Box::new(bound as u8),
                element_identifier: Box::new(element_id),
            })),
            seq_ldefn: None,
            array_sdefn: None,
            array_ldefn: None,
            map_sdefn: None,
            map_ldefn: None,
            sc_component_id: None,
            equivalence_hash: None,
            extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
        }
    } else {
        xt::TypeIdentifier {
            _d: xt::TI_PLAIN_SEQUENCE_LARGE,
            string_sdefn: None,
            string_ldefn: None,
            seq_sdefn: None,
            seq_ldefn: Some(Box::new(xt::PlainSequenceLElemDefn {
                header: Box::new(xt::PlainCollectionHeader {
                    equiv_kind: Box::new(equiv_kind),
                    element_flags: Box::new(xt::CollectionElementFlag { value: 0 }),
                }),
                bound: Box::new(bound),
                element_identifier: Box::new(element_id),
            })),
            array_sdefn: None,
            array_ldefn: None,
            map_sdefn: None,
            map_ldefn: None,
            sc_component_id: None,
            equivalence_hash: None,
            extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
        }
    }
}

pub fn type_identifier_array(
    element_id: xt::TypeIdentifier,
    dims: &[u32],
    eq: TypeEquivalence,
) -> xt::TypeIdentifier {
    let equiv_kind = if is_fully_descriptive(&element_id) {
        xt::EK_BOTH
    } else {
        match eq {
            TypeEquivalence::Complete => xt::EK_COMPLETE,
            TypeEquivalence::Minimal => xt::EK_MINIMAL,
        }
    };
    let use_small = dims.iter().all(|&dim| dim <= u8::MAX as u32);
    if use_small {
        let bounds = dims.iter().map(|&dim| dim as u8).collect::<Vec<_>>();
        xt::TypeIdentifier {
            _d: xt::TI_PLAIN_ARRAY_SMALL,
            string_sdefn: None,
            string_ldefn: None,
            seq_sdefn: None,
            seq_ldefn: None,
            array_sdefn: Some(Box::new(xt::PlainArraySElemDefn {
                header: Box::new(xt::PlainCollectionHeader {
                    equiv_kind: Box::new(equiv_kind),
                    element_flags: Box::new(xt::CollectionElementFlag { value: 0 }),
                }),
                array_bound_seq: Box::new(bounds),
                element_identifier: Box::new(element_id),
            })),
            array_ldefn: None,
            map_sdefn: None,
            map_ldefn: None,
            sc_component_id: None,
            equivalence_hash: None,
            extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
        }
    } else {
        xt::TypeIdentifier {
            _d: xt::TI_PLAIN_ARRAY_LARGE,
            string_sdefn: None,
            string_ldefn: None,
            seq_sdefn: None,
            seq_ldefn: None,
            array_sdefn: None,
            array_ldefn: Some(Box::new(xt::PlainArrayLElemDefn {
                header: Box::new(xt::PlainCollectionHeader {
                    equiv_kind: Box::new(equiv_kind),
                    element_flags: Box::new(xt::CollectionElementFlag { value: 0 }),
                }),
                array_bound_seq: Box::new(dims.to_vec()),
                element_identifier: Box::new(element_id),
            })),
            map_sdefn: None,
            map_ldefn: None,
            sc_component_id: None,
            equivalence_hash: None,
            extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
        }
    }
}

pub fn type_identifier_map(
    key_id: xt::TypeIdentifier,
    element_id: xt::TypeIdentifier,
    bound: u32,
    eq: TypeEquivalence,
) -> xt::TypeIdentifier {
    let equiv_kind = if is_fully_descriptive(&key_id) && is_fully_descriptive(&element_id) {
        xt::EK_BOTH
    } else {
        match eq {
            TypeEquivalence::Complete => xt::EK_COMPLETE,
            TypeEquivalence::Minimal => xt::EK_MINIMAL,
        }
    };
    if bound <= u8::MAX as u32 {
        xt::TypeIdentifier {
            _d: xt::TI_PLAIN_MAP_SMALL,
            string_sdefn: None,
            string_ldefn: None,
            seq_sdefn: None,
            seq_ldefn: None,
            array_sdefn: None,
            array_ldefn: None,
            map_sdefn: Some(Box::new(xt::PlainMapSTypeDefn {
                header: Box::new(xt::PlainCollectionHeader {
                    equiv_kind: Box::new(equiv_kind),
                    element_flags: Box::new(xt::CollectionElementFlag { value: 0 }),
                }),
                bound: Box::new(bound as u8),
                element_identifier: Box::new(element_id),
                key_flags: Box::new(xt::CollectionElementFlag { value: 0 }),
                key_identifier: Box::new(key_id),
            })),
            map_ldefn: None,
            sc_component_id: None,
            equivalence_hash: None,
            extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
        }
    } else {
        xt::TypeIdentifier {
            _d: xt::TI_PLAIN_MAP_LARGE,
            string_sdefn: None,
            string_ldefn: None,
            seq_sdefn: None,
            seq_ldefn: None,
            array_sdefn: None,
            array_ldefn: None,
            map_sdefn: None,
            map_ldefn: Some(Box::new(xt::PlainMapLTypeDefn {
                header: Box::new(xt::PlainCollectionHeader {
                    equiv_kind: Box::new(equiv_kind),
                    element_flags: Box::new(xt::CollectionElementFlag { value: 0 }),
                }),
                bound: Box::new(bound),
                element_identifier: Box::new(element_id),
                key_flags: Box::new(xt::CollectionElementFlag { value: 0 }),
                key_identifier: Box::new(key_id),
            })),
            sc_component_id: None,
            equivalence_hash: None,
            extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
        }
    }
}

pub fn build_complete_alias(
    type_name: &str,
    type_flags: u32,
    related_type: xt::TypeIdentifier,
) -> xt::TypeObject {
    xt::TypeObject {
        _d: xt::EK_COMPLETE,
        complete: Some(Box::new(xt::CompleteTypeObject {
            _d: xt::TK_ALIAS,
            alias_type: Some(Box::new(xt::CompleteAliasType {
                alias_flags: Box::new(xt::TypeFlag { value: type_flags }),
                header: Box::new(xt::CompleteAliasHeader {
                    detail: Box::new(complete_type_detail(type_name)),
                }),
                body: Box::new(xt::CompleteAliasBody {
                    common: Box::new(xt::CommonAliasBody {
                        related_flags: Box::new(xt::MemberFlag { value: 0 }),
                        related_type: Box::new(related_type),
                    }),
                    ann_builtin: Box::new(empty_applied_builtin_member_annotations()),
                    ann_custom: Box::new(Vec::new()),
                }),
            })),
            annotation_type: None,
            struct_type: None,
            union_type: None,
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: None,
            extended_type: None,
        })),
        minimal: None,
    }
}

pub fn build_minimal_alias(type_flags: u32, related_type: xt::TypeIdentifier) -> xt::TypeObject {
    xt::TypeObject {
        _d: xt::EK_MINIMAL,
        complete: None,
        minimal: Some(Box::new(xt::MinimalTypeObject {
            _d: xt::TK_ALIAS,
            alias_type: Some(Box::new(xt::MinimalAliasType {
                alias_flags: Box::new(xt::TypeFlag { value: type_flags }),
                header: Box::new(xt::MinimalAliasHeader {}),
                body: Box::new(xt::MinimalAliasBody {
                    common: Box::new(xt::CommonAliasBody {
                        related_flags: Box::new(xt::MemberFlag { value: 0 }),
                        related_type: Box::new(related_type),
                    }),
                }),
            })),
            annotation_type: None,
            struct_type: None,
            union_type: None,
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: None,
            extended_type: None,
        })),
    }
}

pub fn build_complete_struct(
    type_name: &str,
    type_flags: u32,
    base_type: xt::TypeIdentifier,
    mut members: Vec<StructMemberDesc<'_>>,
) -> xt::TypeObject {
    members.sort_by_key(|member| member.member_id);
    let members = members
        .into_iter()
        .map(|member| xt::CompleteStructMember {
            common: Box::new(xt::CommonStructMember {
                member_id: Box::new(member.member_id),
                member_flags: Box::new(xt::MemberFlag {
                    value: member.member_flags,
                }),
                member_type_id: Box::new(member.type_id),
            }),
            detail: Box::new(complete_member_detail(member.name)),
        })
        .collect::<Vec<_>>();
    xt::TypeObject {
        _d: xt::EK_COMPLETE,
        complete: Some(Box::new(xt::CompleteTypeObject {
            _d: xt::TK_STRUCTURE,
            alias_type: None,
            annotation_type: None,
            struct_type: Some(Box::new(xt::CompleteStructType {
                struct_flags: Box::new(xt::TypeFlag { value: type_flags }),
                header: Box::new(xt::CompleteStructHeader {
                    base_type: Box::new(base_type),
                    detail: Box::new(complete_type_detail(type_name)),
                }),
                member_seq: Box::new(members),
            })),
            union_type: None,
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: None,
            extended_type: None,
        })),
        minimal: None,
    }
}

pub fn build_minimal_struct(
    type_flags: u32,
    base_type: xt::TypeIdentifier,
    mut members: Vec<StructMemberDesc<'_>>,
) -> xt::TypeObject {
    members.sort_by_key(|member| member.member_id);
    let members = members
        .into_iter()
        .map(|member| xt::MinimalStructMember {
            common: Box::new(xt::CommonStructMember {
                member_id: Box::new(member.member_id),
                member_flags: Box::new(xt::MemberFlag {
                    value: member.member_flags,
                }),
                member_type_id: Box::new(member.type_id),
            }),
            detail: Box::new(xt::MinimalMemberDetail {
                name_hash: Box::new(name_hash(member.name)),
            }),
        })
        .collect::<Vec<_>>();
    xt::TypeObject {
        _d: xt::EK_MINIMAL,
        complete: None,
        minimal: Some(Box::new(xt::MinimalTypeObject {
            _d: xt::TK_STRUCTURE,
            alias_type: None,
            annotation_type: None,
            struct_type: Some(Box::new(xt::MinimalStructType {
                struct_flags: Box::new(xt::TypeFlag { value: type_flags }),
                header: Box::new(xt::MinimalStructHeader {
                    base_type: Box::new(base_type),
                    detail: Box::new(xt::MinimalTypeDetail {}),
                }),
                member_seq: Box::new(members),
            })),
            union_type: None,
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: None,
            extended_type: None,
        })),
    }
}

pub fn build_complete_union(
    type_name: &str,
    type_flags: u32,
    discriminator_type: xt::TypeIdentifier,
    mut members: Vec<UnionMemberDesc<'_>>,
) -> xt::TypeObject {
    members.sort_by_key(|member| member.member_id);
    let members = members
        .into_iter()
        .map(|member| xt::CompleteUnionMember {
            common: Box::new(xt::CommonUnionMember {
                member_id: Box::new(member.member_id),
                member_flags: Box::new(xt::UnionMemberFlag {
                    value: member.member_flags,
                }),
                type_id: Box::new(member.type_id),
                label_seq: Box::new(member.labels),
            }),
            detail: Box::new(complete_member_detail(member.name)),
        })
        .collect::<Vec<_>>();
    let discriminator = xt::CompleteDiscriminatorMember {
        common: Box::new(xt::CommonDiscriminatorMember {
            member_flags: Box::new(xt::UnionDiscriminatorFlag { value: 0 }),
            type_id: Box::new(discriminator_type),
        }),
        ann_builtin: Box::new(empty_applied_builtin_type_annotations()),
        ann_custom: Box::new(Vec::new()),
    };
    xt::TypeObject {
        _d: xt::EK_COMPLETE,
        complete: Some(Box::new(xt::CompleteTypeObject {
            _d: xt::TK_UNION,
            alias_type: None,
            annotation_type: None,
            struct_type: None,
            union_type: Some(Box::new(xt::CompleteUnionType {
                union_flags: Box::new(xt::UnionTypeFlag { value: type_flags }),
                header: Box::new(xt::CompleteUnionHeader {
                    detail: Box::new(complete_type_detail(type_name)),
                }),
                discriminator: Box::new(discriminator),
                member_seq: Box::new(members),
            })),
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: None,
            extended_type: None,
        })),
        minimal: None,
    }
}

pub fn build_minimal_union(
    type_flags: u32,
    discriminator_type: xt::TypeIdentifier,
    mut members: Vec<UnionMemberDesc<'_>>,
) -> xt::TypeObject {
    members.sort_by_key(|member| member.member_id);
    let members = members
        .into_iter()
        .map(|member| xt::MinimalUnionMember {
            common: Box::new(xt::CommonUnionMember {
                member_id: Box::new(member.member_id),
                member_flags: Box::new(xt::UnionMemberFlag {
                    value: member.member_flags,
                }),
                type_id: Box::new(member.type_id),
                label_seq: Box::new(member.labels),
            }),
            detail: Box::new(xt::MinimalMemberDetail {
                name_hash: Box::new(name_hash(member.name)),
            }),
        })
        .collect::<Vec<_>>();
    let discriminator = xt::MinimalDiscriminatorMember {
        common: Box::new(xt::CommonDiscriminatorMember {
            member_flags: Box::new(xt::UnionDiscriminatorFlag { value: 0 }),
            type_id: Box::new(discriminator_type),
        }),
    };
    xt::TypeObject {
        _d: xt::EK_MINIMAL,
        complete: None,
        minimal: Some(Box::new(xt::MinimalTypeObject {
            _d: xt::TK_UNION,
            alias_type: None,
            annotation_type: None,
            struct_type: None,
            union_type: Some(Box::new(xt::MinimalUnionType {
                union_flags: Box::new(xt::UnionTypeFlag { value: type_flags }),
                header: Box::new(xt::MinimalUnionHeader {
                    detail: Box::new(xt::MinimalTypeDetail {}),
                }),
                discriminator: Box::new(discriminator),
                member_seq: Box::new(members),
            })),
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: None,
            extended_type: None,
        })),
    }
}

pub fn build_complete_enum(
    type_name: &str,
    type_flags: u32,
    bit_bound: u16,
    mut literals: Vec<EnumLiteralDesc<'_>>,
) -> xt::TypeObject {
    literals.sort_by_key(|literal| literal.value);
    let literals = literals
        .into_iter()
        .map(|literal| xt::CompleteEnumeratedLiteral {
            common: Box::new(xt::CommonEnumeratedLiteral {
                value: literal.value,
                flags: Box::new(xt::EnumeratedLiteralFlag {
                    value: literal.flags,
                }),
            }),
            detail: Box::new(complete_member_detail(literal.name)),
        })
        .collect::<Vec<_>>();
    let header = xt::CompleteEnumeratedHeader {
        common: Box::new(xt::CommonEnumeratedHeader {
            bit_bound: Box::new(bit_bound),
        }),
        detail: Box::new(complete_type_detail(type_name)),
    };
    xt::TypeObject {
        _d: xt::EK_COMPLETE,
        complete: Some(Box::new(xt::CompleteTypeObject {
            _d: xt::TK_ENUM,
            alias_type: None,
            annotation_type: None,
            struct_type: None,
            union_type: None,
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: Some(Box::new(xt::CompleteEnumeratedType {
                enum_flags: Box::new(xt::EnumTypeFlag { value: type_flags }),
                header: Box::new(header),
                literal_seq: Box::new(literals),
            })),
            bitmask_type: None,
            extended_type: None,
        })),
        minimal: None,
    }
}

pub fn build_minimal_enum(
    type_flags: u32,
    bit_bound: u16,
    mut literals: Vec<EnumLiteralDesc<'_>>,
) -> xt::TypeObject {
    literals.sort_by_key(|literal| literal.value);
    let literals = literals
        .into_iter()
        .map(|literal| xt::MinimalEnumeratedLiteral {
            common: Box::new(xt::CommonEnumeratedLiteral {
                value: literal.value,
                flags: Box::new(xt::EnumeratedLiteralFlag {
                    value: literal.flags,
                }),
            }),
            detail: Box::new(xt::MinimalMemberDetail {
                name_hash: Box::new(name_hash(literal.name)),
            }),
        })
        .collect::<Vec<_>>();
    let header = xt::MinimalEnumeratedHeader {
        common: Box::new(xt::CommonEnumeratedHeader {
            bit_bound: Box::new(bit_bound),
        }),
    };
    xt::TypeObject {
        _d: xt::EK_MINIMAL,
        complete: None,
        minimal: Some(Box::new(xt::MinimalTypeObject {
            _d: xt::TK_ENUM,
            alias_type: None,
            annotation_type: None,
            struct_type: None,
            union_type: None,
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: Some(Box::new(xt::MinimalEnumeratedType {
                enum_flags: Box::new(xt::EnumTypeFlag { value: type_flags }),
                header: Box::new(header),
                literal_seq: Box::new(literals),
            })),
            bitmask_type: None,
            extended_type: None,
        })),
    }
}

pub fn build_complete_bitmask(
    type_name: &str,
    type_flags: u32,
    bit_bound: u16,
    mut flags: Vec<BitflagDesc<'_>>,
) -> xt::TypeObject {
    flags.sort_by_key(|flag| flag.position);
    let flags = flags
        .into_iter()
        .map(|flag| xt::CompleteBitflag {
            common: Box::new(xt::CommonBitflag {
                position: flag.position,
                flags: Box::new(xt::BitflagFlag { value: flag.flags }),
            }),
            detail: Box::new(complete_member_detail(flag.name)),
        })
        .collect::<Vec<_>>();
    let header = xt::CompleteEnumeratedHeader {
        common: Box::new(xt::CommonEnumeratedHeader {
            bit_bound: Box::new(bit_bound),
        }),
        detail: Box::new(complete_type_detail(type_name)),
    };
    xt::TypeObject {
        _d: xt::EK_COMPLETE,
        complete: Some(Box::new(xt::CompleteTypeObject {
            _d: xt::TK_BITMASK,
            alias_type: None,
            annotation_type: None,
            struct_type: None,
            union_type: None,
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: Some(Box::new(xt::CompleteBitmaskType {
                bitmask_flags: Box::new(xt::BitmaskTypeFlag { value: type_flags }),
                header: Box::new(header),
                flag_seq: Box::new(flags),
            })),
            extended_type: None,
        })),
        minimal: None,
    }
}

pub fn build_minimal_bitmask(
    type_flags: u32,
    bit_bound: u16,
    mut flags: Vec<BitflagDesc<'_>>,
) -> xt::TypeObject {
    flags.sort_by_key(|flag| flag.position);
    let flags = flags
        .into_iter()
        .map(|flag| xt::MinimalBitflag {
            common: Box::new(xt::CommonBitflag {
                position: flag.position,
                flags: Box::new(xt::BitflagFlag { value: flag.flags }),
            }),
            detail: Box::new(xt::MinimalMemberDetail {
                name_hash: Box::new(name_hash(flag.name)),
            }),
        })
        .collect::<Vec<_>>();
    let header = xt::MinimalEnumeratedHeader {
        common: Box::new(xt::CommonEnumeratedHeader {
            bit_bound: Box::new(bit_bound),
        }),
    };
    xt::TypeObject {
        _d: xt::EK_MINIMAL,
        complete: None,
        minimal: Some(Box::new(xt::MinimalTypeObject {
            _d: xt::TK_BITMASK,
            alias_type: None,
            annotation_type: None,
            struct_type: None,
            union_type: None,
            bitset_type: None,
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: Some(Box::new(xt::MinimalBitmaskType {
                bitmask_flags: Box::new(xt::BitmaskTypeFlag { value: type_flags }),
                header: Box::new(header),
                flag_seq: Box::new(flags),
            })),
            extended_type: None,
        })),
    }
}

pub fn build_complete_bitset(
    type_name: &str,
    type_flags: u32,
    mut fields: Vec<BitfieldDesc<'_>>,
) -> xt::TypeObject {
    fields.sort_by_key(|field| field.position);
    let fields = fields
        .into_iter()
        .map(|field| xt::CompleteBitfield {
            common: Box::new(xt::CommonBitfield {
                position: field.position,
                flags: Box::new(xt::BitsetMemberFlag { value: field.flags }),
                bitcount: field.bitcount,
                holder_type: Box::new(field.holder_type),
            }),
            detail: Box::new(complete_member_detail(field.name.unwrap_or(""))),
        })
        .collect::<Vec<_>>();
    xt::TypeObject {
        _d: xt::EK_COMPLETE,
        complete: Some(Box::new(xt::CompleteTypeObject {
            _d: xt::TK_BITSET,
            alias_type: None,
            annotation_type: None,
            struct_type: None,
            union_type: None,
            bitset_type: Some(Box::new(xt::CompleteBitsetType {
                bitset_flags: Box::new(xt::BitsetTypeFlag { value: type_flags }),
                header: Box::new(xt::CompleteBitsetHeader {
                    detail: Box::new(complete_type_detail(type_name)),
                }),
                field_seq: Box::new(fields),
            })),
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: None,
            extended_type: None,
        })),
        minimal: None,
    }
}

pub fn build_minimal_bitset(type_flags: u32, mut fields: Vec<BitfieldDesc<'_>>) -> xt::TypeObject {
    fields.sort_by_key(|field| field.position);
    let fields = fields
        .into_iter()
        .map(|field| xt::MinimalBitfield {
            common: Box::new(xt::CommonBitfield {
                position: field.position,
                flags: Box::new(xt::BitsetMemberFlag { value: field.flags }),
                bitcount: field.bitcount,
                holder_type: Box::new(field.holder_type),
            }),
            name_hash: Box::new(field.name.map(name_hash).unwrap_or([0u8; 4])),
        })
        .collect::<Vec<_>>();
    xt::TypeObject {
        _d: xt::EK_MINIMAL,
        complete: None,
        minimal: Some(Box::new(xt::MinimalTypeObject {
            _d: xt::TK_BITSET,
            alias_type: None,
            annotation_type: None,
            struct_type: None,
            union_type: None,
            bitset_type: Some(Box::new(xt::MinimalBitsetType {
                bitset_flags: Box::new(xt::BitsetTypeFlag { value: type_flags }),
                header: Box::new(xt::MinimalBitsetHeader {}),
                field_seq: Box::new(fields),
            })),
            sequence_type: None,
            array_type: None,
            map_type: None,
            enumerated_type: None,
            bitmask_type: None,
            extended_type: None,
        })),
    }
}

fn type_identifier_hash(discriminator: u8, hash: [u8; 14]) -> xt::TypeIdentifier {
    xt::TypeIdentifier {
        _d: discriminator,
        string_sdefn: None,
        string_ldefn: None,
        seq_sdefn: None,
        seq_ldefn: None,
        array_sdefn: None,
        array_ldefn: None,
        map_sdefn: None,
        map_ldefn: None,
        sc_component_id: None,
        equivalence_hash: Some(Box::new(hash)),
        extended_defn: Some(Box::new(xt::ExtendedTypeDefn {})),
    }
}

fn is_fully_descriptive(id: &xt::TypeIdentifier) -> bool {
    match id._d {
        xt::TK_BOOLEAN
        | xt::TK_BYTE
        | xt::TK_INT16
        | xt::TK_INT32
        | xt::TK_INT64
        | xt::TK_UINT16
        | xt::TK_UINT32
        | xt::TK_UINT64
        | xt::TK_FLOAT32
        | xt::TK_FLOAT64
        | xt::TK_FLOAT128
        | xt::TK_CHAR8
        | xt::TK_CHAR16 => true,
        xt::TI_STRING8_SMALL
        | xt::TI_STRING8_LARGE
        | xt::TI_STRING16_SMALL
        | xt::TI_STRING16_LARGE => true,
        xt::TI_PLAIN_SEQUENCE_SMALL | xt::TI_PLAIN_SEQUENCE_LARGE => {
            if let Some(seq) = id.seq_sdefn.as_ref() {
                *seq.header.equiv_kind == xt::EK_BOTH
            } else if let Some(seq) = id.seq_ldefn.as_ref() {
                *seq.header.equiv_kind == xt::EK_BOTH
            } else {
                false
            }
        }
        xt::TI_PLAIN_ARRAY_SMALL | xt::TI_PLAIN_ARRAY_LARGE => {
            if let Some(arr) = id.array_sdefn.as_ref() {
                *arr.header.equiv_kind == xt::EK_BOTH
            } else if let Some(arr) = id.array_ldefn.as_ref() {
                *arr.header.equiv_kind == xt::EK_BOTH
            } else {
                false
            }
        }
        xt::TI_PLAIN_MAP_SMALL | xt::TI_PLAIN_MAP_LARGE => {
            if let Some(map) = id.map_sdefn.as_ref() {
                *map.header.equiv_kind == xt::EK_BOTH
            } else if let Some(map) = id.map_ldefn.as_ref() {
                *map.header.equiv_kind == xt::EK_BOTH
            } else {
                false
            }
        }
        _ => false,
    }
}

fn complete_type_detail(name: &str) -> xt::CompleteTypeDetail {
    xt::CompleteTypeDetail {
        ann_builtin: Box::new(empty_applied_builtin_type_annotations()),
        ann_custom: Box::new(Vec::new()),
        type_name: Box::new(name.to_string()),
    }
}

fn complete_member_detail(name: &str) -> xt::CompleteMemberDetail {
    xt::CompleteMemberDetail {
        name: Box::new(name.to_string()),
        ann_builtin: Box::new(empty_applied_builtin_member_annotations()),
        ann_custom: Box::new(Vec::new()),
    }
}

fn empty_applied_builtin_type_annotations() -> xt::AppliedBuiltinTypeAnnotations {
    xt::AppliedBuiltinTypeAnnotations {
        verbatim: Box::new(empty_verbatim_annotation()),
    }
}

fn empty_applied_builtin_member_annotations() -> xt::AppliedBuiltinMemberAnnotations {
    xt::AppliedBuiltinMemberAnnotations {
        unit: String::new(),
        min: Box::new(empty_annotation_parameter_value()),
        max: Box::new(empty_annotation_parameter_value()),
        hash_id: String::new(),
    }
}

fn empty_verbatim_annotation() -> xt::AppliedVerbatimAnnotation {
    xt::AppliedVerbatimAnnotation {
        placement: String::new(),
        language: String::new(),
        text: String::new(),
    }
}

fn empty_annotation_parameter_value() -> xt::AnnotationParameterValue {
    xt::AnnotationParameterValue {
        _d: xt::TK_NONE,
        boolean_value: None,
        byte_value: None,
        int16_value: None,
        uint_16_value: None,
        int32_value: None,
        uint32_value: None,
        int64_value: None,
        uint64_value: None,
        float32_value: None,
        float64_value: None,
        float128_value: None,
        char_value: None,
        wchar_value: None,
        enumerated_value: None,
        string8_value: None,
        string16_value: None,
        extended_value: Some(Box::new(xt::ExtendedAnnotationParameterValue {})),
    }
}

fn hash_complete_type_object(value: &xt::CompleteTypeObject) -> [u8; 14] {
    hash_xcdr2(value)
}

fn hash_minimal_type_object(value: &xt::MinimalTypeObject) -> [u8; 14] {
    hash_xcdr2(value)
}

fn hash_xcdr2<T: xidl_xcdr::XcdrSerialize>(value: &T) -> [u8; 14] {
    let bytes = serialize_xcdr2(value);
    let digest = md5::compute(bytes);
    let mut out = [0u8; 14];
    out.copy_from_slice(&digest.0[..14]);
    out
}

fn serialize_xcdr2<T: xidl_xcdr::XcdrSerialize>(value: &T) -> Vec<u8> {
    let mut size = 256usize;
    loop {
        let mut buf = vec![0u8; size];
        let mut serializer = Xcdr2Serialize::new(buf.as_mut_ptr(), buf.len());
        match value.serialize_with(&mut serializer) {
            Ok(()) => {
                buf.truncate(serializer.pos);
                return buf;
            }
            Err(xidl_xcdr::error::XcdrError::BufferOverflow) => {
                size = size.saturating_mul(2);
                if size == 0 {
                    panic!("xcdr2 serialize: size overflow");
                }
            }
            Err(err) => {
                panic!("xcdr2 serialize: {err}");
            }
        }
    }
}
