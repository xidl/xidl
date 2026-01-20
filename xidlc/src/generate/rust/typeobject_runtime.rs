use crate::generate::rust::util::rust_scoped_name;
use xidl_parser::hir;
use xidl_parser::hir::Annotation;

pub struct TypeObjectCode {
    pub complete: String,
    pub minimal: String,
}

#[derive(Clone, Copy)]
enum EquivalenceKind {
    Complete,
    Minimal,
}

const TYPE_FLAG_IS_FINAL: u32 = 1u32 << 0;
const TYPE_FLAG_IS_APPENDABLE: u32 = 1u32 << 1;
const TYPE_FLAG_IS_MUTABLE: u32 = 1u32 << 2;
const TYPE_FLAG_IS_NESTED: u32 = 1u32 << 3;
const TYPE_FLAG_IS_AUTOID_HASH: u32 = 1u32 << 4;

const MEMBER_FLAG_IS_EXTERNAL: u32 = 1u32 << 2;
const MEMBER_FLAG_IS_KEY: u32 = 1u32 << 5;
const MEMBER_FLAG_IS_DEFAULT: u32 = 1u32 << 6;

pub fn typeobject_for_struct(def: &hir::StructDcl, module_path: &[String]) -> TypeObjectCode {
    let type_name = qualified_name(module_path, &def.ident);
    let type_flags = type_flag_value(&def.annotations);
    let base_complete = def
        .parent
        .first()
        .map(|value| type_identifier_for_scoped_name(value, EquivalenceKind::Complete))
        .unwrap_or_else(type_identifier_none_expr);
    let base_minimal = def
        .parent
        .first()
        .map(|value| type_identifier_for_scoped_name(value, EquivalenceKind::Minimal))
        .unwrap_or_else(type_identifier_none_expr);

    let mut members_complete = Vec::new();
    let mut members_minimal = Vec::new();
    for member in &def.member {
        let member_flags = member_flag_value(&member.annotations, member.default.is_some(), false);
        let member_id = member.field_id.unwrap_or(0);
        for decl in &member.ident {
            let name = declarator_name(decl);
            let type_id_complete = type_identifier_for_declarator(
                &member.ty,
                decl,
                module_path,
                EquivalenceKind::Complete,
            );
            members_complete.push(struct_member_desc(
                member_id,
                member_flags,
                &type_id_complete,
                name,
            ));

            let type_id_minimal = type_identifier_for_declarator(
                &member.ty,
                decl,
                module_path,
                EquivalenceKind::Minimal,
            );
            members_minimal.push(struct_member_desc(
                member_id,
                member_flags,
                &type_id_minimal,
                name,
            ));
        }
    }

    let complete = format!(
        "xidl_typeobject::runtime::build_complete_struct({:?}, {}u32, {}, vec![{}])",
        type_name,
        type_flags,
        base_complete,
        members_complete.join(", ")
    );
    let minimal = format!(
        "xidl_typeobject::runtime::build_minimal_struct({}u32, {}, vec![{}])",
        type_flags,
        base_minimal,
        members_minimal.join(", ")
    );
    TypeObjectCode { complete, minimal }
}

pub fn typeobject_for_union(def: &hir::UnionDef, module_path: &[String]) -> TypeObjectCode {
    let type_name = qualified_name(module_path, &def.ident);
    let type_flags = type_flag_value(&def.annotations);
    let discriminator_complete =
        type_identifier_for_switch_type(&def.switch_type_spec, EquivalenceKind::Complete);
    let discriminator_minimal =
        type_identifier_for_switch_type(&def.switch_type_spec, EquivalenceKind::Minimal);

    let mut member_map: std::collections::HashMap<String, UnionMemberBuild<'_>> =
        std::collections::HashMap::new();
    for case in &def.case {
        let name = declarator_name(&case.element.value).to_string();
        let entry = member_map
            .entry(name.clone())
            .or_insert_with(|| UnionMemberBuild::new(case, name.clone()));
        entry.push_case(case);
    }

    let mut members_complete = Vec::new();
    let mut members_minimal = Vec::new();
    for member in member_map.values() {
        let member_id = member.field_id.unwrap_or(0);
        let member_flags = member_flag_value(member.annotations, false, member.has_default);
        let labels = render_label_seq(&member.label_seq);
        let type_id_complete = type_identifier_for_element_spec(
            &member.element.ty,
            module_path,
            EquivalenceKind::Complete,
        );
        members_complete.push(union_member_desc(
            member_id,
            member_flags,
            &type_id_complete,
            &member.name,
            &labels,
        ));
        let type_id_minimal = type_identifier_for_element_spec(
            &member.element.ty,
            module_path,
            EquivalenceKind::Minimal,
        );
        members_minimal.push(union_member_desc(
            member_id,
            member_flags,
            &type_id_minimal,
            &member.name,
            &labels,
        ));
    }

    let complete = format!(
        "xidl_typeobject::runtime::build_complete_union({:?}, {}u32, {}, vec![{}])",
        type_name,
        type_flags,
        discriminator_complete,
        members_complete.join(", ")
    );
    let minimal = format!(
        "xidl_typeobject::runtime::build_minimal_union({}u32, {}, vec![{}])",
        type_flags,
        discriminator_minimal,
        members_minimal.join(", ")
    );
    TypeObjectCode { complete, minimal }
}

pub fn typeobject_for_enum(def: &hir::EnumDcl, module_path: &[String]) -> TypeObjectCode {
    let type_name = qualified_name(module_path, &def.ident);
    let type_flags = type_flag_value(&def.annotations);
    let bit_bound = enum_bit_bound(&def.annotations).unwrap_or(32);
    let literals = build_enum_literals(def);
    let complete_literals = literals
        .iter()
        .map(|literal| enum_literal_desc(literal, false))
        .collect::<Vec<_>>();
    let minimal_literals = literals
        .iter()
        .map(|literal| enum_literal_desc(literal, true))
        .collect::<Vec<_>>();

    let complete = format!(
        "xidl_typeobject::runtime::build_complete_enum({:?}, {}u32, {}u16, vec![{}])",
        type_name,
        type_flags,
        bit_bound,
        complete_literals.join(", ")
    );
    let minimal = format!(
        "xidl_typeobject::runtime::build_minimal_enum({}u32, {}u16, vec![{}])",
        type_flags,
        bit_bound,
        minimal_literals.join(", ")
    );
    TypeObjectCode { complete, minimal }
}

pub fn typeobject_for_bitmask(def: &hir::BitmaskDcl, module_path: &[String]) -> TypeObjectCode {
    let type_name = qualified_name(module_path, &def.ident);
    let type_flags = type_flag_value(&def.annotations);
    let bit_bound = enum_bit_bound(&def.annotations).unwrap_or(32);
    let flags = build_bitflags(def);
    let complete_flags = flags
        .iter()
        .map(|flag| bitflag_desc(flag, false))
        .collect::<Vec<_>>();
    let minimal_flags = flags
        .iter()
        .map(|flag| bitflag_desc(flag, true))
        .collect::<Vec<_>>();

    let complete = format!(
        "xidl_typeobject::runtime::build_complete_bitmask({:?}, {}u32, {}u16, vec![{}])",
        type_name,
        type_flags,
        bit_bound,
        complete_flags.join(", ")
    );
    let minimal = format!(
        "xidl_typeobject::runtime::build_minimal_bitmask({}u32, {}u16, vec![{}])",
        type_flags,
        bit_bound,
        minimal_flags.join(", ")
    );
    TypeObjectCode { complete, minimal }
}

pub fn typeobject_for_bitset(def: &hir::BitsetDcl, module_path: &[String]) -> TypeObjectCode {
    let type_name = qualified_name(module_path, &def.ident);
    let type_flags = type_flag_value(&def.annotations);
    let mut offset = 0u16;
    let mut complete_fields = Vec::new();
    let mut minimal_fields = Vec::new();
    for field in &def.field {
        let bitcount = positive_int_to_u8(&field.pos).unwrap_or(0);
        let holder = bitfield_holder_type(field);
        let name = if field.ident.is_empty() {
            None
        } else {
            Some(field.ident.as_str())
        };
        complete_fields.push(bitfield_desc(offset, holder, bitcount, name));
        minimal_fields.push(bitfield_desc(offset, holder, bitcount, name));
        offset = offset.saturating_add(bitcount as u16);
    }

    let complete = format!(
        "xidl_typeobject::runtime::build_complete_bitset({:?}, {}u32, vec![{}])",
        type_name,
        type_flags,
        complete_fields.join(", ")
    );
    let minimal = format!(
        "xidl_typeobject::runtime::build_minimal_bitset({}u32, vec![{}])",
        type_flags,
        minimal_fields.join(", ")
    );
    TypeObjectCode { complete, minimal }
}

pub fn typeobject_for_typedef(
    def: &hir::TypedefDcl,
    decl: &hir::Declarator,
    module_path: &[String],
    annotations: &[Annotation],
) -> TypeObjectCode {
    let name = declarator_name(decl);
    let type_name = qualified_name(module_path, name);
    let type_flags = type_flag_value(annotations);
    let base_complete =
        type_identifier_for_typedef_type(&def.ty, decl, module_path, EquivalenceKind::Complete);
    let base_minimal =
        type_identifier_for_typedef_type(&def.ty, decl, module_path, EquivalenceKind::Minimal);
    let complete = format!(
        "xidl_typeobject::runtime::build_complete_alias({:?}, {}u32, {})",
        type_name, type_flags, base_complete
    );
    let minimal = format!(
        "xidl_typeobject::runtime::build_minimal_alias({}u32, {})",
        type_flags, base_minimal
    );
    TypeObjectCode { complete, minimal }
}

fn type_identifier_for_typedef_type(
    ty: &hir::TypedefType,
    decl: &hir::Declarator,
    module_path: &[String],
    eq: EquivalenceKind,
) -> String {
    let base = match ty {
        hir::TypedefType::TypeSpec(value) => {
            type_identifier_for_declarator(value, decl, module_path, eq)
        }
        hir::TypedefType::ConstrTypeDcl(value) => type_identifier_for_constr_type(value, eq),
    };
    let dims = declarator_dims(decl);
    if dims.is_empty() {
        return base;
    }
    let dims_expr = dims
        .iter()
        .map(|dim| format!("{dim}u32"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "xidl_typeobject::runtime::type_identifier_array({}, &[{}], {})",
        base,
        dims_expr,
        eq_expr(eq)
    )
}

fn type_identifier_for_declarator(
    ty: &hir::TypeSpec,
    decl: &hir::Declarator,
    module_path: &[String],
    eq: EquivalenceKind,
) -> String {
    let base = type_identifier_for_spec(ty, module_path, eq);
    let dims = declarator_dims(decl);
    if dims.is_empty() {
        return base;
    }
    let dims_expr = dims
        .iter()
        .map(|dim| format!("{dim}u32"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "xidl_typeobject::runtime::type_identifier_array({}, &[{}], {})",
        base,
        dims_expr,
        eq_expr(eq)
    )
}

fn type_identifier_for_spec(
    ty: &hir::TypeSpec,
    module_path: &[String],
    eq: EquivalenceKind,
) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(value) => match value {
            hir::SimpleTypeSpec::IntegerType(value) => {
                type_identifier_primitive_expr(integer_kind_expr(value))
            }
            hir::SimpleTypeSpec::FloatingPtType => type_identifier_primitive_expr("TK_FLOAT64"),
            hir::SimpleTypeSpec::CharType => type_identifier_primitive_expr("TK_CHAR8"),
            hir::SimpleTypeSpec::WideCharType => type_identifier_primitive_expr("TK_CHAR16"),
            hir::SimpleTypeSpec::Boolean => type_identifier_primitive_expr("TK_BOOLEAN"),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => type_identifier_none_expr(),
            hir::SimpleTypeSpec::ScopedName(value) => type_identifier_for_scoped_name(value, eq),
        },
        hir::TypeSpec::TemplateTypeSpec(value) => match value {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                let element = type_identifier_for_spec(&seq.ty, module_path, eq);
                let bound = seq.len.as_ref().and_then(positive_int_to_u32).unwrap_or(0);
                format!(
                    "xidl_typeobject::runtime::type_identifier_sequence({}, {}u32, {})",
                    element,
                    bound,
                    eq_expr(eq)
                )
            }
            hir::TemplateTypeSpec::StringType(value) => {
                let bound = value.bound.as_ref().and_then(positive_int_to_u32);
                format!(
                    "xidl_typeobject::runtime::type_identifier_string({}, false)",
                    bound_expr(bound)
                )
            }
            hir::TemplateTypeSpec::WideStringType(value) => {
                let bound = value.bound.as_ref().and_then(positive_int_to_u32);
                format!(
                    "xidl_typeobject::runtime::type_identifier_string({}, true)",
                    bound_expr(bound)
                )
            }
            hir::TemplateTypeSpec::MapType(map) => {
                let key_id = type_identifier_for_spec(&map.key, module_path, eq);
                let value_id = type_identifier_for_spec(&map.value, module_path, eq);
                let bound = map.len.as_ref().and_then(positive_int_to_u32).unwrap_or(0);
                format!(
                    "xidl_typeobject::runtime::type_identifier_map({}, {}, {}u32, {})",
                    key_id,
                    value_id,
                    bound,
                    eq_expr(eq)
                )
            }
            hir::TemplateTypeSpec::FixedPtType(_) => type_identifier_none_expr(),
        },
    }
}

fn type_identifier_for_element_spec(
    ty: &hir::ElementSpecTy,
    module_path: &[String],
    eq: EquivalenceKind,
) -> String {
    match ty {
        hir::ElementSpecTy::TypeSpec(value) => type_identifier_for_spec(value, module_path, eq),
        hir::ElementSpecTy::ConstrTypeDcl(value) => type_identifier_for_constr_type(value, eq),
    }
}

fn type_identifier_for_constr_type(value: &hir::ConstrTypeDcl, eq: EquivalenceKind) -> String {
    let name = constr_type_name(value);
    match name {
        Some(name) => format!(
            "xidl_typeobject::runtime::type_identifier_for::<{}>({})",
            rust_scoped_name(&name),
            eq_expr(eq)
        ),
        None => type_identifier_none_expr(),
    }
}

fn type_identifier_for_switch_type(value: &hir::SwitchTypeSpec, eq: EquivalenceKind) -> String {
    match value {
        hir::SwitchTypeSpec::IntegerType(value) => {
            type_identifier_primitive_expr(integer_kind_expr(value))
        }
        hir::SwitchTypeSpec::CharType => type_identifier_primitive_expr("TK_CHAR8"),
        hir::SwitchTypeSpec::WideCharType => type_identifier_primitive_expr("TK_CHAR16"),
        hir::SwitchTypeSpec::BooleanType => type_identifier_primitive_expr("TK_BOOLEAN"),
        hir::SwitchTypeSpec::ScopedName(value) => type_identifier_for_scoped_name(value, eq),
        hir::SwitchTypeSpec::OctetType => type_identifier_primitive_expr("TK_BYTE"),
    }
}

fn type_identifier_for_scoped_name(name: &hir::ScopedName, eq: EquivalenceKind) -> String {
    format!(
        "xidl_typeobject::runtime::type_identifier_for::<{}>({})",
        rust_scoped_name(name),
        eq_expr(eq)
    )
}

fn type_identifier_primitive_expr(kind: &str) -> String {
    format!(
        "xidl_typeobject::runtime::type_identifier_primitive(xidl_typeobject::DDS::XTypes::{kind})"
    )
}

fn type_identifier_none_expr() -> String {
    "xidl_typeobject::runtime::type_identifier_none()".to_string()
}

fn eq_expr(eq: EquivalenceKind) -> &'static str {
    match eq {
        EquivalenceKind::Complete => "xidl_typeobject::runtime::TypeEquivalence::Complete",
        EquivalenceKind::Minimal => "xidl_typeobject::runtime::TypeEquivalence::Minimal",
    }
}

fn integer_kind_expr(value: &hir::IntegerType) -> &'static str {
    match value {
        hir::IntegerType::Char => "TK_BYTE",
        hir::IntegerType::UChar => "TK_BYTE",
        hir::IntegerType::U8 => "TK_BYTE",
        hir::IntegerType::U16 => "TK_UINT16",
        hir::IntegerType::U32 => "TK_UINT32",
        hir::IntegerType::U64 => "TK_UINT64",
        hir::IntegerType::I8 => "TK_BYTE",
        hir::IntegerType::I16 => "TK_INT16",
        hir::IntegerType::I32 => "TK_INT32",
        hir::IntegerType::I64 => "TK_INT64",
    }
}

fn bitfield_holder_type(field: &hir::BitField) -> &'static str {
    match field.ty {
        Some(hir::BitFieldType::Bool) => "TK_BOOLEAN",
        Some(hir::BitFieldType::Octec) => "TK_BYTE",
        Some(hir::BitFieldType::SignedInt) => "TK_INT32",
        Some(hir::BitFieldType::UnsignedInt) => "TK_UINT32",
        None => "TK_UINT32",
    }
}

fn bound_expr(bound: Option<u32>) -> String {
    match bound {
        Some(value) => format!("Some({value}u32)"),
        None => "None".to_string(),
    }
}

fn type_flag_value(annotations: &[Annotation]) -> u32 {
    let ext = hir::extensibility_from_annotations(annotations);
    let mut value = match ext {
        hir::Extensibility::Final => TYPE_FLAG_IS_FINAL,
        hir::Extensibility::Appendable => TYPE_FLAG_IS_APPENDABLE,
        hir::Extensibility::Mutable => TYPE_FLAG_IS_MUTABLE,
        hir::Extensibility::None => 0,
    };
    for anno in annotations {
        if let Annotation::Builtin { name, params } = anno {
            if name.eq_ignore_ascii_case("nested") {
                value |= TYPE_FLAG_IS_NESTED;
            }
            if name.eq_ignore_ascii_case("autoid") {
                if let Some(value_raw) = annotation_param_raw(params) {
                    if value_raw.eq_ignore_ascii_case("hash") {
                        value |= TYPE_FLAG_IS_AUTOID_HASH;
                    }
                }
            }
        }
    }
    value
}

fn member_flag_value(annotations: &[Annotation], has_default: bool, is_default: bool) -> u32 {
    let mut value = 0u32;
    if has_default {
        value |= MEMBER_FLAG_IS_DEFAULT;
    }
    if is_default {
        value |= MEMBER_FLAG_IS_DEFAULT;
    }
    for anno in annotations {
        match anno {
            Annotation::Key => value |= MEMBER_FLAG_IS_KEY,
            Annotation::Builtin { name, .. } => {
                if name.eq_ignore_ascii_case("external") {
                    value |= MEMBER_FLAG_IS_EXTERNAL;
                }
            }
            _ => {}
        }
    }
    value
}

fn annotation_param_raw(params: &Option<hir::AnnotationParams>) -> Option<String> {
    match params {
        Some(hir::AnnotationParams::Raw(value)) => Some(value.trim().trim_matches('"').to_string()),
        Some(hir::AnnotationParams::ConstExpr(expr)) => {
            hir::const_expr_to_i64(expr).map(|value| value.to_string())
        }
        Some(hir::AnnotationParams::Params(params)) => params
            .iter()
            .find(|param| param.ident.eq_ignore_ascii_case("value"))
            .and_then(|param| param.value.as_ref())
            .and_then(hir::const_expr_to_i64)
            .map(|value| value.to_string()),
        None => None,
    }
}

fn enum_bit_bound(annotations: &[Annotation]) -> Option<u16> {
    for anno in annotations {
        if let Annotation::Builtin { name, params } = anno {
            if name.eq_ignore_ascii_case("bit_bound") {
                if let Some(raw) = annotation_param_raw(params) {
                    if let Ok(value) = raw.parse::<u16>() {
                        return Some(value);
                    }
                }
            }
        }
    }
    None
}

fn struct_member_desc(member_id: u32, member_flags: u32, type_id: &str, name: &str) -> String {
    format!(
        "xidl_typeobject::runtime::StructMemberDesc {{ member_id: {member_id}u32, member_flags: {member_flags}u32, type_id: {type_id}, name: {:?} }}",
        name
    )
}

fn union_member_desc(
    member_id: u32,
    member_flags: u32,
    type_id: &str,
    name: &str,
    labels: &str,
) -> String {
    format!(
        "xidl_typeobject::runtime::UnionMemberDesc {{ member_id: {member_id}u32, member_flags: {member_flags}u32, type_id: {type_id}, name: {:?}, labels: {labels} }}",
        name
    )
}

fn enum_literal_desc(value: &EnumLiteralBuild, minimal: bool) -> String {
    let _ = minimal;
    let flags = value.flags;
    format!(
        "xidl_typeobject::runtime::EnumLiteralDesc {{ value: {}i32, flags: {}u32, name: {:?} }}",
        value.value, flags, value.name
    )
}

fn bitflag_desc(value: &BitflagBuild, minimal: bool) -> String {
    let flags = if minimal { 0 } else { 0 };
    format!(
        "xidl_typeobject::runtime::BitflagDesc {{ position: {}u16, flags: {}u32, name: {:?} }}",
        value.position, flags, value.name
    )
}

fn bitfield_desc(position: u16, holder: &str, bitcount: u8, name: Option<&str>) -> String {
    let name_value = match name {
        Some(value) if !value.is_empty() => format!("Some({:?})", value),
        _ => "None".to_string(),
    };
    format!(
        "xidl_typeobject::runtime::BitfieldDesc {{ position: {position}u16, flags: 0u32, bitcount: {bitcount}u8, holder_type: xidl_typeobject::DDS::XTypes::{holder}, name: {name_value} }}"
    )
}

struct EnumLiteralBuild {
    name: String,
    value: i32,
    flags: u32,
}

struct BitflagBuild {
    name: String,
    position: u16,
}

struct UnionMemberBuild<'a> {
    name: String,
    annotations: &'a [Annotation],
    element: &'a hir::ElementSpec,
    field_id: Option<u32>,
    label_seq: Vec<i32>,
    has_default: bool,
}

impl<'a> UnionMemberBuild<'a> {
    fn new(case: &'a hir::Case, name: String) -> Self {
        Self {
            name,
            annotations: &case.element.annotations,
            element: &case.element,
            field_id: case.element.field_id,
            label_seq: Vec::new(),
            has_default: false,
        }
    }

    fn push_case(&mut self, case: &'a hir::Case) {
        for label in &case.label {
            match label {
                hir::CaseLabel::Default => self.has_default = true,
                hir::CaseLabel::Value(expr) => {
                    let value = hir::const_expr_to_i64(expr).unwrap_or(0);
                    self.label_seq.push(value as i32);
                }
            }
        }
    }
}

fn render_label_seq(values: &[i32]) -> String {
    if values.is_empty() {
        "vec![]".to_string()
    } else {
        let labels = values
            .iter()
            .map(|value| format!("{value}i32"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("vec![{labels}]")
    }
}

fn build_enum_literals(def: &hir::EnumDcl) -> Vec<EnumLiteralBuild> {
    let mut out = Vec::new();
    let mut next_value = 0i32;
    for member in &def.member {
        let mut value = None;
        let mut flags = 0u32;
        for anno in &member.annotations {
            if let Annotation::Builtin { name, params } = anno {
                if name.eq_ignore_ascii_case("value") {
                    if let Some(raw) = annotation_param_raw(params) {
                        value = raw.parse::<i32>().ok();
                    }
                }
                if name.eq_ignore_ascii_case("default") {
                    flags |= MEMBER_FLAG_IS_DEFAULT;
                }
            }
        }
        let value = value.unwrap_or(next_value);
        next_value = value.saturating_add(1);
        out.push(EnumLiteralBuild {
            name: member.ident.clone(),
            value,
            flags,
        });
    }
    out
}

fn build_bitflags(def: &hir::BitmaskDcl) -> Vec<BitflagBuild> {
    let mut out = Vec::new();
    let mut next_pos = 0u16;
    for value in &def.value {
        let mut position = None;
        for anno in &value.annotations {
            if let Annotation::Builtin { name, params } = anno {
                if name.eq_ignore_ascii_case("position") {
                    if let Some(raw) = annotation_param_raw(params) {
                        position = raw.parse::<u16>().ok();
                    }
                }
            }
        }
        let position = position.unwrap_or(next_pos);
        next_pos = position.saturating_add(1);
        out.push(BitflagBuild {
            name: value.ident.clone(),
            position,
        });
    }
    out
}

fn constr_type_name(constr: &hir::ConstrTypeDcl) -> Option<hir::ScopedName> {
    let name = match constr {
        hir::ConstrTypeDcl::StructForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::StructDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::EnumDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionDef(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitsetDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitmaskDcl(def) => def.ident.clone(),
    };
    Some(hir::ScopedName {
        name: vec![name],
        is_root: false,
    })
}

fn declarator_name(decl: &hir::Declarator) -> &str {
    match decl {
        hir::Declarator::SimpleDeclarator(value) => &value.0,
        hir::Declarator::ArrayDeclarator(value) => &value.ident,
    }
}

fn declarator_dims(decl: &hir::Declarator) -> Vec<u32> {
    match decl {
        hir::Declarator::SimpleDeclarator(_) => Vec::new(),
        hir::Declarator::ArrayDeclarator(value) => {
            value.len.iter().filter_map(positive_int_to_u32).collect()
        }
    }
}

fn positive_int_to_u32(value: &hir::PositiveIntConst) -> Option<u32> {
    hir::const_expr_to_i64(&value.0).and_then(|value| {
        if value >= 0 && value <= u32::MAX as i64 {
            Some(value as u32)
        } else {
            None
        }
    })
}

fn positive_int_to_u8(value: &hir::PositiveIntConst) -> Option<u8> {
    hir::const_expr_to_i64(&value.0).and_then(|value| {
        if value >= 0 && value <= u8::MAX as i64 {
            Some(value as u8)
        } else {
            None
        }
    })
}

fn qualified_name(module_path: &[String], ident: &str) -> String {
    let mut full = String::new();
    for name in module_path {
        if !full.is_empty() {
            full.push_str("::");
        }
        full.push_str(name);
    }
    if !full.is_empty() {
        full.push_str("::");
    }
    full.push_str(ident);
    full
}
