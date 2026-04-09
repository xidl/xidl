use serde_json::{Value, json};
use xidl_parser::hir;

fn schema_ref(name: &str) -> Value {
    json!({ "$ref": format!("#/components/schemas/{name}") })
}

pub(super) fn schema_for_type(ty: &hir::TypeSpec) -> Value {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => integer_schema(value),
            hir::SimpleTypeSpec::FloatingPtType => json!({ "type": "number", "format": "double" }),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => {
                json!({ "type": "string" })
            }
            hir::SimpleTypeSpec::Boolean => json!({ "type": "boolean" }),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => json!({}),
            hir::SimpleTypeSpec::ScopedName(value) => match value.name.join(".").as_str() {
                "dds.rpc.UnusedMember" => {
                    json!({ "type": "object", "properties": {}, "required": [] })
                }
                "dds.rpc.RequestHeader" | "dds.rpc.ReplyHeader" => {
                    json!({ "type": "object", "additionalProperties": true })
                }
                "dds.rpc.UnknownOperation" => json!({ "type": "object" }),
                scoped => schema_ref(scoped),
            },
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                let mut out = json!({ "type": "array", "items": schema_for_type(&seq.ty) });
                if let Some(len) = &seq.len {
                    if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
                        if size >= 0 {
                            out["minItems"] = Value::Number((size as u64).into());
                            out["maxItems"] = Value::Number((size as u64).into());
                        }
                    }
                }
                out
            }
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                json!({ "type": "string" })
            }
            hir::TemplateTypeSpec::FixedPtType(_) => {
                json!({ "type": "number", "format": "double" })
            }
            hir::TemplateTypeSpec::MapType(map) => json!({
                "type": "object",
                "additionalProperties": schema_for_type(&map.value),
            }),
            hir::TemplateTypeSpec::TemplateType(_) => json!({ "type": "object" }),
        },
    }
}

fn integer_schema(value: &hir::IntegerType) -> Value {
    match value {
        hir::IntegerType::U64 => json!({ "type": "integer", "format": "int64", "minimum": 0 }),
        hir::IntegerType::U32
        | hir::IntegerType::U16
        | hir::IntegerType::U8
        | hir::IntegerType::UChar => {
            json!({ "type": "integer", "format": "int32", "minimum": 0 })
        }
        hir::IntegerType::I64 => json!({ "type": "integer", "format": "int64" }),
        _ => json!({ "type": "integer", "format": "int32" }),
    }
}

pub(super) fn is_internal_rpc_marker_type(ty: &hir::TypeSpec) -> bool {
    matches!(
        ty,
        hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(value))
            if matches!(value.name.join(".").as_str(), "dds.rpc.UnusedMember" | "dds.rpc.RequestHeader" | "dds.rpc.ReplyHeader")
    )
}

pub(super) fn is_internal_rpc_marker_element(ty: &hir::ElementSpecTy) -> bool {
    match ty {
        hir::ElementSpecTy::TypeSpec(spec) => {
            matches!(
                spec,
                hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(value))
                    if value.name.join(".") == "dds.rpc.UnknownOperation"
            )
        }
        hir::ElementSpecTy::ConstrTypeDcl(_) => false,
    }
}
