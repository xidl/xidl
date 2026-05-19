use xidl_parser::hir;

use super::method::TypeRefTarget;
use super::names::{
    declarator_name, integer_schema, ts_ident, ts_prop_name, ts_scoped_name, zod_schema_ref,
};

pub(crate) fn ts_type_for_decl(
    ty: &hir::TypeSpec,
    decl: &hir::Declarator,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
    apply_array_ts(ts_type_for_type_spec(ty, module_path, target), decl)
}

pub(crate) fn ts_type_for_element(
    ty: &hir::ElementSpecTy,
    decl: &hir::Declarator,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
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
) -> String {
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
                    fields.push(if optional {
                        format!("{}?: {ty}", ts_prop_name(&name))
                    } else {
                        format!("{}: {ty}", ts_prop_name(&name))
                    });
                }
            }
            format!("{{ {} }}", fields.join(", "))
        }
        hir::ConstrTypeDcl::EnumDcl(def) => def
            .member
            .iter()
            .map(|value| {
                let raw =
                    hir::effective_wire_name(&value.ident, &value.annotations, &def.annotations);
                format!("\"{raw}\"")
            })
            .collect::<Vec<_>>()
            .join(" | "),
        hir::ConstrTypeDcl::UnionDef(def) => inline_union_ts(def, module_path, target),
        hir::ConstrTypeDcl::BitsetDcl(_) | hir::ConstrTypeDcl::BitmaskDcl(_) => "number".into(),
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {
            "unknown".into()
        }
    }
}

fn inline_union_ts(def: &hir::UnionDef, module_path: &[String], target: TypeRefTarget) -> String {
    let variants = def
        .case
        .iter()
        .map(|case| {
            let name = ts_prop_name(declarator_name(&case.element.value));
            let ty =
                ts_type_for_element(&case.element.ty, &case.element.value, module_path, target);
            format!("{{ {name}: {ty} }}")
        })
        .collect::<Vec<_>>();
    if variants.is_empty() {
        "never".to_string()
    } else {
        variants.join(" | ")
    }
}

pub(crate) fn zod_schema_for_decl(
    ty: &hir::TypeSpec,
    decl: &hir::Declarator,
    module_path: &[String],
) -> String {
    apply_array_zod(zod_schema_for_type_spec(ty, module_path), decl)
}

pub(crate) fn zod_schema_for_element(
    ty: &hir::ElementSpecTy,
    decl: &hir::Declarator,
    module_path: &[String],
) -> String {
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
) -> String {
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
                    fields.push(if optional {
                        format!("{}: {schema}.optional()", ts_prop_name(&name))
                    } else {
                        format!("{}: {schema}", ts_prop_name(&name))
                    });
                }
            }
            format!("z.object({{ {} }})", fields.join(", "))
        }
        hir::ConstrTypeDcl::EnumDcl(def) => inline_enum_zod(def),
        hir::ConstrTypeDcl::UnionDef(def) => inline_union_zod(def, module_path),
        hir::ConstrTypeDcl::BitsetDcl(_) | hir::ConstrTypeDcl::BitmaskDcl(_) => {
            "z.number().int()".into()
        }
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {
            "z.unknown()".into()
        }
    }
}

fn inline_enum_zod(def: &hir::EnumDcl) -> String {
    let values = def
        .member
        .iter()
        .map(|value| {
            let raw = hir::effective_wire_name(&value.ident, &value.annotations, &def.annotations);
            format!("\"{raw}\"")
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        "z.never()".to_string()
    } else {
        format!("z.enum([{}])", values.join(", "))
    }
}

fn inline_union_zod(def: &hir::UnionDef, module_path: &[String]) -> String {
    let variants = def
        .case
        .iter()
        .map(|case| {
            let name = ts_prop_name(declarator_name(&case.element.value));
            let schema = zod_schema_for_element(&case.element.ty, &case.element.value, module_path);
            format!("z.object({{ {name}: {schema} }})")
        })
        .collect::<Vec<_>>();
    match variants.len() {
        0 => "z.never()".to_string(),
        1 => variants[0].clone(),
        _ => format!("z.union([{}])", variants.join(", ")),
    }
}

pub(crate) fn ts_type_for_type_spec(
    ty: &hir::TypeSpec,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
    match ty {
        hir::TypeSpec::IntegerType(_) | hir::TypeSpec::FloatingPtType => "number".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "string".to_string(),
        hir::TypeSpec::Boolean => "boolean".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "unknown".to_string()
        }
        hir::TypeSpec::ScopedName(value) => ts_scoped_name(value, module_path, target),
        hir::TypeSpec::SequenceType(seq) => {
            format!(
                "Array<{}>",
                ts_type_for_type_spec(&seq.ty, module_path, target)
            )
        }
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "string".to_string(),
        hir::TypeSpec::FixedPtType(_) => "number".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!(
                "Record<string, {}>",
                ts_type_for_type_spec(&map.value, module_path, target)
            )
        }
        hir::TypeSpec::TemplateType(value) => ts_template_type(value, module_path, target),
    }
}

pub(crate) fn zod_schema_for_type_spec(ty: &hir::TypeSpec, module_path: &[String]) -> String {
    match ty {
        hir::TypeSpec::IntegerType(value) => integer_schema(value),
        hir::TypeSpec::FloatingPtType => "z.number()".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "z.string()".to_string(),
        hir::TypeSpec::Boolean => "z.boolean()".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "z.unknown()".to_string()
        }
        hir::TypeSpec::ScopedName(value) => zod_schema_ref(value, module_path),
        hir::TypeSpec::SequenceType(seq) => length_limited_array(
            format!(
                "z.array({})",
                zod_schema_for_type_spec(&seq.ty, module_path)
            ),
            seq.len
                .as_ref()
                .and_then(|len| xidl_parser::hir::const_expr_to_i64(&len.0)),
        ),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "z.string()".to_string(),
        hir::TypeSpec::FixedPtType(_) => "z.number()".to_string(),
        hir::TypeSpec::MapType(map) => {
            format!(
                "z.record({})",
                zod_schema_for_type_spec(&map.value, module_path)
            )
        }
        hir::TypeSpec::TemplateType(value) => {
            let ty = ts_template_type(value, module_path, TypeRefTarget::Types);
            format!("z.custom<{ty}>()")
        }
    }
}

fn ts_template_type(
    value: &hir::TemplateType,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
    let args = value
        .args
        .iter()
        .map(|arg| ts_type_for_type_spec(arg, module_path, target))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}<{args}>", ts_ident(&value.ident))
}

pub(crate) fn apply_array_ts(mut base: String, decl: &hir::Declarator) -> String {
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for _ in &array.len {
            base = format!("Array<{base}>");
        }
    }
    base
}

pub(crate) fn apply_array_zod(mut base: String, decl: &hir::Declarator) -> String {
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for len in &array.len {
            base = length_limited_array(
                format!("z.array({base})"),
                xidl_parser::hir::const_expr_to_i64(&len.0),
            );
        }
    }
    base
}

fn length_limited_array(base: String, len: Option<i64>) -> String {
    match len {
        Some(size) if size >= 0 => format!("{base}.length({size})"),
        _ => base,
    }
}
