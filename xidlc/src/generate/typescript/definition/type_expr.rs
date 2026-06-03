use xidl_parser::hir;

use super::contexts::{FieldTypeContext, FieldZodContext, TsType, ZodSchema};
use super::method::TypeRefTarget;
use super::names::{
    declarator_name, integer_schema_primitive, ts_ident, ts_prop_name, ts_scoped_name,
    zod_schema_ref,
};

pub(crate) fn ts_type_for_decl(
    ty: &hir::TypeSpec,
    decl: &hir::Declarator,
    module_path: &[String],
    target: TypeRefTarget,
) -> TsType {
    apply_array_ts(ts_type_for_type_spec(ty, module_path, target), decl)
}

pub(crate) fn ts_type_for_element(
    ty: &hir::ElementSpecTy,
    decl: &hir::Declarator,
    module_path: &[String],
    target: TypeRefTarget,
) -> TsType {
    let base = match ty {
        hir::ElementSpecTy::TypeSpec(spec) => ts_type_for_type_spec(spec, module_path, target),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => {
            ts_type_for_constr_inline(constr, module_path, target)
        }
    };
    apply_array_ts(base, decl)
}

pub(crate) fn ts_type_for_constr_inline(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
    target: TypeRefTarget,
) -> TsType {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            let mut fields = Vec::new();
            for member in &def.member {
                let optional = member.is_optional();
                for decl in &member.ident {
                    let name = hir::effective_wire_name(
                        declarator_name(decl),
                        &member.annotations,
                        &def.annotations,
                    );
                    let ty = ts_type_for_decl(&member.ty, decl, module_path, target);
                    fields.push(FieldTypeContext {
                        prop: ts_prop_name(&name),
                        ty,
                        optional,
                        doc: Vec::new(),
                    });
                }
            }
            TsType::InlineStruct(fields)
        }
        hir::ConstrTypeDcl::EnumDcl(def) => TsType::InlineEnum(
            def.member
                .iter()
                .map(|value| {
                    hir::effective_wire_name(&value.ident, &value.annotations, &def.annotations)
                })
                .collect(),
        ),
        hir::ConstrTypeDcl::UnionDef(def) => inline_union_ts(def, module_path, target),
        hir::ConstrTypeDcl::BitsetDcl(_) | hir::ConstrTypeDcl::BitmaskDcl(_) => {
            TsType::Primitive("number".into())
        }
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {
            TsType::Any
        }
    }
}

fn inline_union_ts(def: &hir::UnionDef, module_path: &[String], target: TypeRefTarget) -> TsType {
    let variants = def
        .case
        .iter()
        .map(|case| {
            let name = ts_prop_name(declarator_name(&case.element.value));
            let ty =
                ts_type_for_element(&case.element.ty, &case.element.value, module_path, target);
            TsType::InlineStruct(vec![FieldTypeContext {
                prop: name,
                ty,
                optional: false,
                doc: Vec::new(),
            }])
        })
        .collect::<Vec<_>>();
    if variants.is_empty() {
        TsType::Void
    } else if variants.len() == 1 {
        variants[0].clone()
    } else {
        TsType::Union(variants)
    }
}

pub(crate) fn zod_schema_for_decl(
    ty: &hir::TypeSpec,
    decl: &hir::Declarator,
    module_path: &[String],
) -> ZodSchema {
    apply_array_zod(zod_schema_for_type_spec(ty, module_path), decl)
}

pub(crate) fn zod_schema_for_element(
    ty: &hir::ElementSpecTy,
    decl: &hir::Declarator,
    module_path: &[String],
) -> ZodSchema {
    let base = match ty {
        hir::ElementSpecTy::TypeSpec(spec) => zod_schema_for_type_spec(spec, module_path),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => {
            zod_schema_for_constr_inline(constr, module_path)
        }
    };
    apply_array_zod(base, decl)
}

pub(crate) fn zod_schema_for_constr_inline(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
) -> ZodSchema {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            let mut fields = Vec::new();
            for member in &def.member {
                let optional = member.is_optional();
                for decl in &member.ident {
                    let name = hir::effective_wire_name(
                        declarator_name(decl),
                        &member.annotations,
                        &def.annotations,
                    );
                    let schema = zod_schema_for_decl(&member.ty, decl, module_path);
                    fields.push(FieldZodContext {
                        prop: ts_prop_name(&name),
                        schema,
                        optional,
                        xjson_meta: None,
                    });
                }
            }
            ZodSchema::Object(fields)
        }
        hir::ConstrTypeDcl::EnumDcl(def) => inline_enum_zod(def),
        hir::ConstrTypeDcl::UnionDef(def) => inline_union_zod(def, module_path),
        hir::ConstrTypeDcl::BitsetDcl(_) | hir::ConstrTypeDcl::BitmaskDcl(_) => {
            ZodSchema::Primitive("number().int()".into())
        }
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {
            ZodSchema::Any
        }
    }
}

fn inline_enum_zod(def: &hir::EnumDcl) -> ZodSchema {
    let values = def
        .member
        .iter()
        .map(|value| hir::effective_wire_name(&value.ident, &value.annotations, &def.annotations))
        .collect::<Vec<_>>();
    if values.is_empty() {
        ZodSchema::Never
    } else {
        ZodSchema::Enum(values)
    }
}

fn inline_union_zod(def: &hir::UnionDef, module_path: &[String]) -> ZodSchema {
    let variants = def
        .case
        .iter()
        .map(|case| {
            let name = ts_prop_name(declarator_name(&case.element.value));
            let schema = zod_schema_for_element(&case.element.ty, &case.element.value, module_path);
            FieldZodContext {
                prop: name,
                schema,
                optional: false,
                xjson_meta: None,
            }
        })
        .collect::<Vec<_>>();
    match variants.len() {
        0 => ZodSchema::Never,
        1 => ZodSchema::Object(vec![variants[0].clone()]),
        _ => ZodSchema::Union(
            variants
                .into_iter()
                .map(|v| ZodSchema::Object(vec![v]))
                .collect(),
        ),
    }
}

pub(crate) fn ts_type_for_type_spec(
    ty: &hir::TypeSpec,
    module_path: &[String],
    target: TypeRefTarget,
) -> TsType {
    match ty {
        hir::TypeSpec::IntegerType(_) | hir::TypeSpec::FixedPtType(_) => {
            TsType::Primitive("number".to_string())
        }
        hir::TypeSpec::FloatingPtType => TsType::Primitive("number".to_string()),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => {
            TsType::Primitive("string".to_string())
        }
        hir::TypeSpec::Boolean => TsType::Primitive("boolean".to_string()),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            TsType::Any
        }
        hir::TypeSpec::ScopedName(value) => {
            TsType::ScopedName(ts_scoped_name(value, module_path, target))
        }
        hir::TypeSpec::SequenceType(seq) => {
            TsType::Sequence(Box::new(ts_type_for_type_spec(&seq.ty, module_path, target)))
        }
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => {
            TsType::Primitive("string".to_string())
        }
        hir::TypeSpec::MapType(map) => {
            TsType::Map(Box::new(ts_type_for_type_spec(&map.value, module_path, target)))
        }
        hir::TypeSpec::TemplateType(value) => ts_template_type(value, module_path, target),
    }
}

pub(crate) fn zod_schema_for_type_spec(ty: &hir::TypeSpec, module_path: &[String]) -> ZodSchema {
    match ty {
        hir::TypeSpec::IntegerType(value) => ZodSchema::Primitive(integer_schema_primitive(value)),
        hir::TypeSpec::FloatingPtType => ZodSchema::Primitive("coerce.number()".to_string()),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => {
            ZodSchema::Primitive("string()".to_string())
        }
        hir::TypeSpec::Boolean => ZodSchema::Primitive("coerce.boolean()".to_string()),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            ZodSchema::Any
        }
        hir::TypeSpec::ScopedName(value) => ZodSchema::ScopedName(zod_schema_ref(value, module_path)),
        hir::TypeSpec::SequenceType(seq) => ZodSchema::Array {
            element: Box::new(zod_schema_for_type_spec(&seq.ty, module_path)),
            length: seq
                .len
                .as_ref()
                .and_then(|len| xidl_parser::hir::const_expr_to_i64(&len.0)),
        },
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => {
            ZodSchema::Primitive("coerce.string()".to_string())
        }
        hir::TypeSpec::FixedPtType(_) => ZodSchema::Primitive("number()".to_string()),
        hir::TypeSpec::MapType(map) => {
            ZodSchema::Record(Box::new(zod_schema_for_type_spec(&map.value, module_path)))
        }
        hir::TypeSpec::TemplateType(value) => {
            let ty = ts_template_type(value, module_path, TypeRefTarget::Types);
            ZodSchema::Custom(ty)
        }
    }
}

fn ts_template_type(
    value: &hir::TemplateType,
    module_path: &[String],
    target: TypeRefTarget,
) -> TsType {
    let args = value
        .args
        .iter()
        .map(|arg| ts_type_for_type_spec(arg, module_path, target))
        .collect::<Vec<_>>();
    TsType::Template {
        ident: ts_ident(&value.ident),
        args,
    }
}

pub(crate) fn apply_array_ts(mut base: TsType, decl: &hir::Declarator) -> TsType {
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for _ in &array.len {
            base = TsType::Sequence(Box::new(base));
        }
    }
    base
}

pub(crate) fn apply_array_zod(mut base: ZodSchema, decl: &hir::Declarator) -> ZodSchema {
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for len in &array.len {
            base = ZodSchema::Array {
                element: Box::new(base),
                length: xidl_parser::hir::const_expr_to_i64(&len.0),
            };
        }
    }
    base
}
