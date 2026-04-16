use convert_case::{Case, Casing};
use xidl_parser::hir;
use xidl_parser::hir::TypeSpec;

pub(super) fn py_type(value: &TypeSpec) -> String {
    match value {
        TypeSpec::SimpleTypeSpec(value) => match value {
            hir::SimpleTypeSpec::IntegerType(hir::IntegerType::U8)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::U16)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::U32)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::U64)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::I8)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::I16)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::I32)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::I64)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::Char)
            | hir::SimpleTypeSpec::IntegerType(hir::IntegerType::UChar) => "int".to_string(),
            hir::SimpleTypeSpec::FloatingPtType => "float".to_string(),
            hir::SimpleTypeSpec::CharType
            | hir::SimpleTypeSpec::WideCharType
            | hir::SimpleTypeSpec::ScopedName(hir::ScopedName { .. }) => match value {
                hir::SimpleTypeSpec::ScopedName(name) => py_scoped_name(name),
                _ => "str".to_string(),
            },
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
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
                    format!(
                        "{}[{}]",
                        py_type_name(&value.ident),
                        value
                            .args
                            .iter()
                            .map(py_type)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        },
    }
}

pub(super) fn py_const_type(value: &hir::ConstType) -> String {
    match value {
        hir::ConstType::IntegerType(_) | hir::ConstType::OctetType => "int".to_string(),
        hir::ConstType::FloatingPtType | hir::ConstType::FixedPtConstType => "float".to_string(),
        hir::ConstType::CharType
        | hir::ConstType::WideCharType
        | hir::ConstType::StringType(_)
        | hir::ConstType::WideStringType(_) => "str".to_string(),
        hir::ConstType::BooleanType => "bool".to_string(),
        hir::ConstType::ScopedName(value) => py_scoped_name(value),
        hir::ConstType::SequenceType(value) => format!("list[{}]", py_type(&value.ty)),
    }
}

pub(super) fn py_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(expr, &py_scoped_name, &|literal| match literal {
        hir::Literal::IntegerLiteral(value) => value.0.clone(),
        hir::Literal::FloatingPtLiteral(value) => {
            let sign = value
                .sign
                .as_ref()
                .map(hir::IntegerSign::as_str)
                .unwrap_or("");
            format!("{sign}{}.{}", value.integer.0, value.fraction.0)
        }
        hir::Literal::CharLiteral(value)
        | hir::Literal::WideCharacterLiteral(value)
        | hir::Literal::StringLiteral(value)
        | hir::Literal::WideStringLiteral(value) => format!("{value:?}"),
        hir::Literal::BooleanLiteral(value) => {
            if *value {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
    })
}

pub(super) fn optional_type(optional: bool, ty: &str) -> String {
    if optional {
        format!("Optional[{ty}]")
    } else {
        ty.to_string()
    }
}

pub(super) fn default_value(optional: bool, default: Option<&hir::Default>, ty: &str) -> String {
    if optional {
        "None".to_string()
    } else if let Some(default) = default {
        py_const_expr(&default.0)
    } else if ty.starts_with("list[") {
        "field(default_factory=list)".to_string()
    } else if ty.starts_with("dict[") {
        "field(default_factory=dict)".to_string()
    } else {
        match ty {
            "int" => "0".to_string(),
            "float" => "0.0".to_string(),
            "bool" => "False".to_string(),
            "str" => "\"\"".to_string(),
            _ => "None".to_string(),
        }
    }
}

pub(super) fn py_switch_type(value: &hir::SwitchTypeSpec) -> String {
    match value {
        hir::SwitchTypeSpec::IntegerType(_) | hir::SwitchTypeSpec::OctetType => "int".to_string(),
        hir::SwitchTypeSpec::CharType | hir::SwitchTypeSpec::WideCharType => "str".to_string(),
        hir::SwitchTypeSpec::BooleanType => "bool".to_string(),
        hir::SwitchTypeSpec::ScopedName(value) => py_scoped_name(value),
    }
}

pub(super) fn py_scoped_name(value: &hir::ScopedName) -> String {
    value
        .name
        .iter()
        .map(|part| py_type_name(part))
        .collect::<Vec<_>>()
        .join("_")
}

pub(super) fn py_type_name(value: &str) -> String {
    value.to_case(Case::Pascal)
}

pub(super) fn py_field_name(value: &str) -> String {
    value.to_case(Case::Snake)
}

pub(super) fn py_const_name(value: &str) -> String {
    value.to_case(Case::UpperSnake)
}
