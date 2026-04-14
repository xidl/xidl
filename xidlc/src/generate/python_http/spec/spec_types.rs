use convert_case::{Case, Casing};
use xidl_parser::hir;
use xidl_parser::hir::TypeSpec;

pub(super) fn py_type_name(value: &str) -> String {
    value.to_case(Case::Pascal)
}

pub(super) fn py_field_name(value: &str) -> String {
    value.to_case(Case::Snake)
}

pub(super) fn maybe_optional_type(optional: bool, ty: &str) -> String {
    if optional {
        format!("Optional[{ty}]")
    } else {
        ty.to_string()
    }
}

pub(super) fn py_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

pub(super) fn py_type(value: &TypeSpec) -> String {
    match value {
        TypeSpec::SimpleTypeSpec(value) => match value {
            hir::SimpleTypeSpec::IntegerType(_) => "int".to_string(),
            hir::SimpleTypeSpec::FloatingPtType => "float".to_string(),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => "str".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => value
                .name
                .iter()
                .map(|part| py_type_name(part))
                .collect::<Vec<_>>()
                .join("_"),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => "Any".to_string(),
        },
        TypeSpec::TemplateTypeSpec(value) => match value {
            hir::TemplateTypeSpec::SequenceType(value) => format!("list[{}]", py_type(&value.ty)),
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                "str".to_string()
            }
            hir::TemplateTypeSpec::FixedPtType(_) => "float".to_string(),
            hir::TemplateTypeSpec::MapType(value) => {
                format!("dict[{}, {}]", py_type(&value.key), py_type(&value.value))
            }
            hir::TemplateTypeSpec::TemplateType(value) => {
                if value.args.is_empty() {
                    py_type_name(&value.ident)
                } else {
                    "Any".to_string()
                }
            }
        },
    }
}
