use convert_case::{Case, Casing};
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
        TypeSpec::IntegerType(_) => "int".to_string(),
        TypeSpec::FloatingPtType => "float".to_string(),
        TypeSpec::CharType | TypeSpec::WideCharType => "str".to_string(),
        TypeSpec::Boolean => "bool".to_string(),
        TypeSpec::ScopedName(value) => value
            .name
            .iter()
            .map(|part| py_type_name(part))
            .collect::<Vec<_>>()
            .join("_"),
        TypeSpec::AnyType | TypeSpec::ObjectType | TypeSpec::ValueBaseType => "Any".to_string(),
        TypeSpec::SequenceType(value) => format!("list[{}]", py_type(&value.ty)),
        TypeSpec::StringType(_) | TypeSpec::WideStringType(_) => "str".to_string(),
        TypeSpec::FixedPtType(_) => "float".to_string(),
        TypeSpec::MapType(value) => {
            format!("dict[{}, {}]", py_type(&value.key), py_type(&value.value))
        }
        TypeSpec::TemplateType(value) => {
            if value.args.is_empty() {
                py_type_name(&value.ident)
            } else {
                "Any".to_string()
            }
        }
    }
}
