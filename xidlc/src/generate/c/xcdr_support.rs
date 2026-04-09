use crate::generate::c::util::{c_literal, c_scoped_name, c_scoped_name_hir};
use crate::generate::render_const_expr;
use serde_json::json;
use xidl_parser::hir;

#[derive(Clone)]
pub(crate) enum FieldKind {
    Primitive {
        serialize_fn: &'static str,
        deserialize_fn: &'static str,
    },
    Custom {
        type_name: String,
    },
    Unsupported {
        reason: String,
    },
}

pub(crate) fn type_kind(ty: &hir::TypeSpec) -> FieldKind {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => integer_kind(value),
            hir::SimpleTypeSpec::FloatingPtType => {
                primitive("cdr_serializer_write_f64", "cdr_deserializer_read_f64_le")
            }
            hir::SimpleTypeSpec::CharType => {
                primitive("cdr_serializer_write_i8", "cdr_deserializer_read_i8")
            }
            hir::SimpleTypeSpec::WideCharType => {
                primitive("cdr_serializer_write_u32", "cdr_deserializer_read_u32_le")
            }
            hir::SimpleTypeSpec::Boolean => {
                primitive("cdr_serializer_write_bool", "cdr_deserializer_read_bool")
            }
            hir::SimpleTypeSpec::ScopedName(value) => FieldKind::Custom {
                type_name: c_scoped_name_hir(value),
            },
            _ => unsupported("unsupported simple type"),
        },
        hir::TypeSpec::TemplateTypeSpec(_) => unsupported("unsupported template type"),
    }
}

pub(crate) fn switch_kind(ty: &hir::SwitchTypeSpec) -> FieldKind {
    match ty {
        hir::SwitchTypeSpec::IntegerType(value) => integer_kind(value),
        hir::SwitchTypeSpec::CharType => {
            primitive("cdr_serializer_write_i8", "cdr_deserializer_read_i8")
        }
        hir::SwitchTypeSpec::WideCharType => {
            primitive("cdr_serializer_write_u32", "cdr_deserializer_read_u32_le")
        }
        hir::SwitchTypeSpec::BooleanType => {
            primitive("cdr_serializer_write_bool", "cdr_deserializer_read_bool")
        }
        hir::SwitchTypeSpec::OctetType => {
            primitive("cdr_serializer_write_u8", "cdr_deserializer_read_u8")
        }
        hir::SwitchTypeSpec::ScopedName(value) => FieldKind::Custom {
            type_name: c_scoped_name_hir(value),
        },
    }
}

pub(crate) fn element_kind(ty: &hir::ElementSpecTy) -> FieldKind {
    match ty {
        hir::ElementSpecTy::TypeSpec(spec) => type_kind(spec),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => FieldKind::Custom {
            type_name: constr_type_name(constr),
        },
    }
}

pub(crate) fn constr_type_name(constr: &hir::ConstrTypeDcl) -> String {
    match constr {
        hir::ConstrTypeDcl::StructForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::StructDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::EnumDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionDef(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitsetDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitmaskDcl(def) => def.ident.clone(),
    }
}

pub(crate) fn type_kind_from_c(ty: &str) -> FieldKind {
    match ty {
        "uint8_t" => primitive("cdr_serializer_write_u8", "cdr_deserializer_read_u8"),
        "int8_t" => primitive("cdr_serializer_write_i8", "cdr_deserializer_read_i8"),
        "uint16_t" => primitive("cdr_serializer_write_u16", "cdr_deserializer_read_u16_le"),
        "int16_t" => primitive("cdr_serializer_write_i16", "cdr_deserializer_read_i16_le"),
        "uint32_t" => primitive("cdr_serializer_write_u32", "cdr_deserializer_read_u32_le"),
        "int32_t" => primitive("cdr_serializer_write_i32", "cdr_deserializer_read_i32_le"),
        "uint64_t" => primitive("cdr_serializer_write_u64", "cdr_deserializer_read_u64_le"),
        "int64_t" => primitive("cdr_serializer_write_i64", "cdr_deserializer_read_i64_le"),
        "bool" => primitive("cdr_serializer_write_bool", "cdr_deserializer_read_bool"),
        _ => unsupported(&format!("unsupported bitfield type: {ty}")),
    }
}

pub(crate) fn declarator_info(decl: &hir::Declarator) -> (String, Vec<String>) {
    match decl {
        hir::Declarator::SimpleDeclarator(value) => (value.0.clone(), Vec::new()),
        hir::Declarator::ArrayDeclarator(value) => {
            let dims = value
                .len
                .iter()
                .map(|len| render_const_expr(&len.0, &c_scoped_name, &c_literal))
                .collect();
            (value.ident.clone(), dims)
        }
    }
}

pub(crate) fn kind_json(kind: &FieldKind) -> serde_json::Value {
    match kind {
        FieldKind::Primitive {
            serialize_fn,
            deserialize_fn,
        } => json!({
            "kind": "primitive",
            "serialize_fn": serialize_fn,
            "deserialize_fn": deserialize_fn,
        }),
        FieldKind::Custom { type_name } => json!({
            "kind": "custom",
            "type_name": type_name,
        }),
        FieldKind::Unsupported { reason } => json!({
            "kind": "unsupported",
            "reason": reason,
        }),
    }
}

fn integer_kind(value: &hir::IntegerType) -> FieldKind {
    match value {
        hir::IntegerType::Char | hir::IntegerType::I8 => {
            primitive("cdr_serializer_write_i8", "cdr_deserializer_read_i8")
        }
        hir::IntegerType::UChar | hir::IntegerType::U8 => {
            primitive("cdr_serializer_write_u8", "cdr_deserializer_read_u8")
        }
        hir::IntegerType::U16 => {
            primitive("cdr_serializer_write_u16", "cdr_deserializer_read_u16_le")
        }
        hir::IntegerType::U32 => {
            primitive("cdr_serializer_write_u32", "cdr_deserializer_read_u32_le")
        }
        hir::IntegerType::U64 => {
            primitive("cdr_serializer_write_u64", "cdr_deserializer_read_u64_le")
        }
        hir::IntegerType::I16 => {
            primitive("cdr_serializer_write_i16", "cdr_deserializer_read_i16_le")
        }
        hir::IntegerType::I32 => {
            primitive("cdr_serializer_write_i32", "cdr_deserializer_read_i32_le")
        }
        hir::IntegerType::I64 => {
            primitive("cdr_serializer_write_i64", "cdr_deserializer_read_i64_le")
        }
    }
}

fn primitive(serialize_fn: &'static str, deserialize_fn: &'static str) -> FieldKind {
    FieldKind::Primitive {
        serialize_fn,
        deserialize_fn,
    }
}

fn unsupported(reason: &str) -> FieldKind {
    FieldKind::Unsupported {
        reason: reason.to_string(),
    }
}
