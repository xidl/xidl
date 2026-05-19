use std::fmt::Write;
use xidl_parser::hir;

use super::spec_types::{
    default_value, optional_type, py_const_expr, py_const_name, py_const_type, py_field_name,
    py_switch_type, py_type, py_type_name,
};
pub(super) fn render_constr_type(out: &mut String, constr: &hir::ConstrTypeDcl) {
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
        hir::ConstrTypeDcl::StructForwardDcl(value) => render_forward_decl(out, &value.ident),
        hir::ConstrTypeDcl::UnionForwardDcl(value) => render_forward_decl(out, &value.ident),
    }
}

pub(super) fn render_type_decl(out: &mut String, type_dcl: &hir::TypeDcl) {
    match type_dcl {
        hir::TypeDcl::ConstrTypeDcl(value) => render_constr_type(out, value),
        hir::TypeDcl::TypedefDcl(value) => {
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
        hir::TypeDcl::NativeDcl(value) => {
            writeln!(out, "{} = Any", py_type_name(&value.decl.0)).unwrap();
            writeln!(out).unwrap();
        }
    }
}

pub(super) fn render_struct(out: &mut String, value: &hir::StructDcl) {
    writeln!(out, "@dataclass").unwrap();
    writeln!(out, "class {}:", py_type_name(&value.ident)).unwrap();
    let members: Vec<_> = value
        .member
        .iter()
        .filter(|m| !hir::is_skipped(&m.annotations))
        .collect();
    if members.is_empty() {
        writeln!(out, "    pass").unwrap();
    } else {
        for member in members {
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

pub(super) fn render_enum(out: &mut String, value: &hir::EnumDcl) {
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

pub(super) fn render_const(out: &mut String, value: &hir::ConstDcl) {
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

pub(super) fn render_exception(out: &mut String, value: &hir::ExceptDcl) {
    writeln!(out, "@dataclass").unwrap();
    writeln!(out, "class {}(Exception):", py_type_name(&value.ident)).unwrap();
    let members: Vec<_> = value
        .member
        .iter()
        .filter(|m| !hir::is_skipped(&m.annotations))
        .collect();
    if members.is_empty() {
        writeln!(out, "    message: str = \"\"").unwrap();
    } else {
        for member in members {
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

fn render_forward_decl(out: &mut String, ident: &str) {
    writeln!(out, "class {}:", py_type_name(ident)).unwrap();
    writeln!(out, "    pass").unwrap();
    writeln!(out).unwrap();
}
