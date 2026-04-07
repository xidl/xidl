use crate::error::IdlcResult;
use crate::generate::python::PythonRenderer;
use convert_case::{Case, Casing};
use serde::Serialize;
use std::fmt::Write;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, TypeSpec};

#[derive(Serialize)]
struct PythonSpecTemplate {
    module_name: String,
    body: String,
}

pub(crate) fn render_spec(
    spec: &hir::Specification,
    module_name: &str,
    _properties: &ParserProperties,
) -> IdlcResult<String> {
    let renderer = PythonRenderer::new()?;
    let mut body = String::new();
    for def in &spec.0 {
        render_definition(&mut body, def, &[])?;
    }
    renderer.render_template(
        "spec.py.j2",
        &PythonSpecTemplate {
            module_name: module_name.to_string(),
            body,
        },
    )
}

fn render_definition(out: &mut String, def: &hir::Definition, prefix: &[String]) -> IdlcResult<()> {
    match def {
        hir::Definition::ModuleDcl(module) => {
            let mut next = prefix.to_vec();
            next.push(module.ident.clone());
            writeln!(out, "# module {}", next.join(".")).unwrap();
            for inner in &module.definition {
                render_definition(out, inner, &next)?;
            }
        }
        hir::Definition::ConstDcl(const_dcl) => render_const(out, const_dcl),
        hir::Definition::ExceptDcl(except_dcl) => render_exception(out, except_dcl),
        hir::Definition::InterfaceDcl(interface) => render_interface(out, interface),
        hir::Definition::TypeDcl(type_dcl) => render_type_decl(out, type_dcl),
        hir::Definition::ConstrTypeDcl(constr) => render_constr_type(out, constr),
        hir::Definition::Pragma(_) => {}
    }
    Ok(())
}

fn render_constr_type(out: &mut String, constr: &hir::ConstrTypeDcl) {
    match constr {
        hir::ConstrTypeDcl::StructDcl(value) => render_struct(out, value),
        hir::ConstrTypeDcl::EnumDcl(value) => render_enum(out, value),
        hir::ConstrTypeDcl::UnionDef(value) => {
            writeln!(out, "@dataclass").unwrap();
            writeln!(out, "class {}:", py_type_name(&value.ident)).unwrap();
            writeln!(
                out,
                "    discriminator: {}",
                py_switch_type(&value.switch_type_spec)
            )
            .unwrap();
            writeln!(out, "    value: Any").unwrap();
            writeln!(out).unwrap();
        }
        hir::ConstrTypeDcl::BitmaskDcl(value) => {
            writeln!(out, "class {}(enum.Flag):", py_type_name(&value.ident)).unwrap();
            if value.value.is_empty() {
                writeln!(out, "    pass").unwrap();
            } else {
                for (index, member) in value.value.iter().enumerate() {
                    writeln!(
                        out,
                        "    {} = {}",
                        py_const_name(&member.ident),
                        1usize << index
                    )
                    .unwrap();
                }
            }
            writeln!(out).unwrap();
        }
        hir::ConstrTypeDcl::BitsetDcl(value) => {
            writeln!(out, "@dataclass").unwrap();
            writeln!(out, "class {}:", py_type_name(&value.ident)).unwrap();
            if value.field.is_empty() {
                writeln!(out, "    pass").unwrap();
            } else {
                for field in &value.field {
                    writeln!(out, "    {}: int = 0", py_field_name(&field.ident)).unwrap();
                }
            }
            writeln!(out).unwrap();
        }
        hir::ConstrTypeDcl::StructForwardDcl(value) => {
            writeln!(out, "class {}:", py_type_name(&value.ident)).unwrap();
            writeln!(out, "    pass").unwrap();
            writeln!(out).unwrap();
        }
        hir::ConstrTypeDcl::UnionForwardDcl(value) => {
            writeln!(out, "class {}:", py_type_name(&value.ident)).unwrap();
            writeln!(out, "    pass").unwrap();
            writeln!(out).unwrap();
        }
    }
}

fn render_type_decl(out: &mut String, type_dcl: &hir::TypeDcl) {
    match &type_dcl.decl {
        hir::TypeDclInner::ConstrTypeDcl(value) => render_constr_type(out, value),
        hir::TypeDclInner::TypedefDcl(value) => {
            let ty = match &value.ty {
                hir::TypedefType::TypeSpec(value) => py_type(value),
                hir::TypedefType::ConstrTypeDcl(value) => {
                    render_constr_type(out, value);
                    match value {
                        hir::ConstrTypeDcl::StructDcl(value) => py_type_name(&value.ident),
                        hir::ConstrTypeDcl::EnumDcl(value) => py_type_name(&value.ident),
                        hir::ConstrTypeDcl::UnionDef(value) => py_type_name(&value.ident),
                        hir::ConstrTypeDcl::BitmaskDcl(value) => py_type_name(&value.ident),
                        hir::ConstrTypeDcl::BitsetDcl(value) => py_type_name(&value.ident),
                        hir::ConstrTypeDcl::StructForwardDcl(value) => py_type_name(&value.ident),
                        hir::ConstrTypeDcl::UnionForwardDcl(value) => py_type_name(&value.ident),
                    }
                }
            };
            for decl in &value.decl {
                match decl {
                    hir::Declarator::SimpleDeclarator(name) => {
                        writeln!(out, "{} = {}", py_type_name(&name.0), ty).unwrap();
                    }
                    hir::Declarator::ArrayDeclarator(name) => {
                        writeln!(out, "{} = list[{}]", py_type_name(&name.ident), ty).unwrap();
                    }
                }
            }
            writeln!(out).unwrap();
        }
        hir::TypeDclInner::NativeDcl(value) => {
            writeln!(out, "{} = Any", py_type_name(&value.decl.0)).unwrap();
            writeln!(out).unwrap();
        }
    }
}

fn render_struct(out: &mut String, value: &hir::StructDcl) {
    writeln!(out, "@dataclass").unwrap();
    writeln!(out, "class {}:", py_type_name(&value.ident)).unwrap();
    if value.member.is_empty() {
        writeln!(out, "    pass").unwrap();
    } else {
        for member in &value.member {
            let member_ty = py_type(&member.ty);
            for declarator in &member.ident {
                match declarator {
                    hir::Declarator::SimpleDeclarator(name) => {
                        writeln!(
                            out,
                            "    {}: {} = {}",
                            py_field_name(&name.0),
                            optional_type(member.is_optional(), &member_ty),
                            default_value(
                                member.is_optional(),
                                member.default.as_ref(),
                                &member_ty
                            )
                        )
                        .unwrap();
                    }
                    hir::Declarator::ArrayDeclarator(name) => {
                        writeln!(
                            out,
                            "    {}: list[{}] = field(default_factory=list)",
                            py_field_name(&name.ident),
                            member_ty
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
    writeln!(out).unwrap();
}

fn render_enum(out: &mut String, value: &hir::EnumDcl) {
    writeln!(out, "class {}(enum.Enum):", py_type_name(&value.ident)).unwrap();
    if value.member.is_empty() {
        writeln!(out, "    pass").unwrap();
    } else {
        for member in &value.member {
            writeln!(
                out,
                "    {} = \"{}\"",
                py_const_name(&member.ident),
                member.ident
            )
            .unwrap();
        }
    }
    writeln!(out).unwrap();
}

fn render_const(out: &mut String, value: &hir::ConstDcl) {
    writeln!(
        out,
        "{}: {} = {}",
        py_const_name(&value.ident),
        py_const_type(&value.ty),
        py_const_expr(&value.value)
    )
    .unwrap();
    writeln!(out).unwrap();
}

fn render_exception(out: &mut String, value: &hir::ExceptDcl) {
    writeln!(out, "@dataclass").unwrap();
    writeln!(out, "class {}(Exception):", py_type_name(&value.ident)).unwrap();
    if value.member.is_empty() {
        writeln!(out, "    message: str = \"\"").unwrap();
    } else {
        for member in &value.member {
            let member_ty = py_type(&member.ty);
            for declarator in &member.ident {
                match declarator {
                    hir::Declarator::SimpleDeclarator(name) => {
                        writeln!(out, "    {}: {}", py_field_name(&name.0), member_ty).unwrap();
                    }
                    hir::Declarator::ArrayDeclarator(name) => {
                        writeln!(
                            out,
                            "    {}: list[{}]",
                            py_field_name(&name.ident),
                            member_ty
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
    writeln!(out).unwrap();
}

fn render_interface(out: &mut String, value: &hir::InterfaceDcl) {
    let hir::InterfaceDclInner::InterfaceDef(def) = &value.decl else {
        return;
    };
    writeln!(out, "class {}(abc.ABC):", py_type_name(&def.header.ident)).unwrap();
    let body = def.interface_body.as_ref().map(|body| &body.0);
    if body.map(|items| items.is_empty()).unwrap_or(true) {
        writeln!(out, "    pass").unwrap();
        writeln!(out).unwrap();
        return;
    }
    for export in body.unwrap() {
        if let hir::Export::OpDcl(op) = export {
            let ret = match &op.ty {
                hir::OpTypeSpec::Void => "None".to_string(),
                hir::OpTypeSpec::TypeSpec(value) => py_type(value),
            };
            let params = op
                .parameter
                .as_ref()
                .map(|params| {
                    params
                        .0
                        .iter()
                        .map(|param| {
                            format!(
                                "{}: {}",
                                py_field_name(&param.declarator.0),
                                py_type(&param.ty)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let suffix = if params.is_empty() {
                "self".to_string()
            } else {
                format!("self, {params}")
            };
            writeln!(out, "    @abc.abstractmethod").unwrap();
            writeln!(
                out,
                "    def {}({}) -> {}:",
                py_field_name(&op.ident),
                suffix,
                ret
            )
            .unwrap();
            writeln!(out, "        raise NotImplementedError").unwrap();
            writeln!(out).unwrap();
        }
    }
}

fn py_type(value: &TypeSpec) -> String {
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

fn py_const_type(value: &hir::ConstType) -> String {
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

fn py_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(expr, &py_scoped_name, &|literal| match literal {
        hir::Literal::IntegerLiteral(value) => match value {
            hir::IntegerLiteral::BinNumber(value)
            | hir::IntegerLiteral::OctNumber(value)
            | hir::IntegerLiteral::DecNumber(value)
            | hir::IntegerLiteral::HexNumber(value) => value.clone(),
        },
        hir::Literal::FloatingPtLiteral(value) => {
            let sign = value
                .sign
                .as_ref()
                .map(|value| value.0.as_str())
                .unwrap_or("");
            format!("{sign}{}.{}", value.integer.0, value.fraction.0)
        }
        hir::Literal::CharLiteral(value)
        | hir::Literal::WideCharacterLiteral(value)
        | hir::Literal::StringLiteral(value)
        | hir::Literal::WideStringLiteral(value) => format!("{value:?}"),
        hir::Literal::BooleanLiteral(value) => {
            if value.eq_ignore_ascii_case("true") {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
    })
}

fn optional_type(optional: bool, ty: &str) -> String {
    if optional {
        format!("Optional[{ty}]")
    } else {
        ty.to_string()
    }
}

fn default_value(optional: bool, default: Option<&hir::Default>, ty: &str) -> String {
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

fn py_switch_type(value: &hir::SwitchTypeSpec) -> String {
    match value {
        hir::SwitchTypeSpec::IntegerType(_) | hir::SwitchTypeSpec::OctetType => "int".to_string(),
        hir::SwitchTypeSpec::CharType | hir::SwitchTypeSpec::WideCharType => "str".to_string(),
        hir::SwitchTypeSpec::BooleanType => "bool".to_string(),
        hir::SwitchTypeSpec::ScopedName(value) => py_scoped_name(value),
    }
}

fn py_scoped_name(value: &hir::ScopedName) -> String {
    value
        .name
        .iter()
        .map(|part| py_type_name(part))
        .collect::<Vec<_>>()
        .join("_")
}

fn py_type_name(value: &str) -> String {
    value.to_case(Case::Pascal)
}

fn py_field_name(value: &str) -> String {
    value.to_case(Case::Snake)
}

fn py_const_name(value: &str) -> String {
    value.to_case(Case::UpperSnake)
}
