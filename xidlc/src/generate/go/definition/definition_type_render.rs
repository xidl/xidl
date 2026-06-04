use crate::error::IdlcResult;
use crate::generate::go::GoRenderContext;
use convert_case::{Case, Casing};
use std::fmt::Write;
use xidl_parser::hir::{self, effective_wire_name};

use super::definition_names::{go_export_name, go_field_name, pointer_type};
use super::definition_support::declarator_name;
use super::definition_types::{constr_type_name, type_with_decl};

pub(super) fn render_type_dcl(
    ctx: &mut GoRenderContext,
    def: &hir::TypeDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    match def {
        hir::TypeDcl::ConstrTypeDcl(constr) => render_constr_type(ctx, constr, prefix),
        hir::TypeDcl::TypedefDcl(typedef) => render_typedef(ctx, typedef, prefix),
        hir::TypeDcl::NativeDcl(native) => {
            let name = go_export_name(prefix, &native.decl.0);
            writeln!(&mut ctx.body, "type {name} any").unwrap();
            writeln!(&mut ctx.body).unwrap();
            Ok(())
        }
    }
}

pub(super) fn render_exception(
    ctx: &mut GoRenderContext,
    def: &hir::ExceptDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    render_field_struct(ctx, &name, &def.member, &[]);
    writeln!(&mut ctx.body, "func (e *{name}) Error() string {{").unwrap();
    writeln!(&mut ctx.body, "\treturn \"{name}\"").unwrap();
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_typedef(
    ctx: &mut GoRenderContext,
    def: &hir::TypedefDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    match &def.ty {
        hir::TypedefType::TypeSpec(ty) => {
            for decl in &def.decl {
                let name = go_field_name(declarator_name(decl));
                let base = type_with_decl(ty, decl);
                writeln!(
                    &mut ctx.body,
                    "type {} = {}",
                    go_export_name(prefix, &name),
                    base
                )
                .unwrap();
                writeln!(&mut ctx.body).unwrap();
            }
        }
        hir::TypedefType::ConstrTypeDcl(constr) => {
            render_constr_type(ctx, constr, prefix)?;
            let base = constr_type_name(constr, prefix);
            for decl in &def.decl {
                writeln!(
                    &mut ctx.body,
                    "type {} = {}",
                    go_export_name(prefix, declarator_name(decl)),
                    base
                )
                .unwrap();
                writeln!(&mut ctx.body).unwrap();
            }
        }
    }
    Ok(())
}

fn render_constr_type(
    ctx: &mut GoRenderContext,
    constr: &hir::ConstrTypeDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => render_struct(ctx, def, prefix),
        hir::ConstrTypeDcl::EnumDcl(def) => render_enum(ctx, def, prefix),
        hir::ConstrTypeDcl::UnionDef(def) => render_simple_struct(
            ctx,
            &go_export_name(prefix, &def.ident),
            &[("Tag", "string", "tag"), ("Value", "any", ",flatten")],
        ),
        hir::ConstrTypeDcl::BitsetDcl(def) => render_simple_struct(
            ctx,
            &go_export_name(prefix, &def.ident),
            &[("Bits", "uint64", "bits")],
        ),
        hir::ConstrTypeDcl::BitmaskDcl(def) => {
            writeln!(
                &mut ctx.body,
                "type {} uint64",
                go_export_name(prefix, &def.ident)
            )
            .unwrap();
            writeln!(&mut ctx.body).unwrap();
            Ok(())
        }
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => Ok(()),
    }
}

fn render_struct(
    ctx: &mut GoRenderContext,
    def: &hir::StructDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    render_field_struct(
        ctx,
        &go_export_name(prefix, &def.ident),
        &def.member,
        &def.annotations,
    );
    Ok(())
}

fn render_enum(ctx: &mut GoRenderContext, def: &hir::EnumDcl, prefix: &[String]) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    writeln!(&mut ctx.body, "type {name} string").unwrap();
    writeln!(&mut ctx.body).unwrap();
    writeln!(&mut ctx.body, "const (").unwrap();
    for member in &def.member {
        let value_name = format!("{}{}", name, member.ident.to_case(Case::Pascal));
        let wire_name = effective_wire_name(&member.ident, &member.annotations, &def.annotations);
        writeln!(&mut ctx.body, "\t{value_name} {name} = \"{wire_name}\"").unwrap();
    }
    writeln!(&mut ctx.body, ")").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_simple_struct(
    ctx: &mut GoRenderContext,
    name: &str,
    fields: &[(&str, &str, &str)],
) -> IdlcResult<()> {
    writeln!(&mut ctx.body, "type {name} struct {{").unwrap();
    for (field, ty, tag) in fields {
        writeln!(&mut ctx.body, "\t{field} {ty} `xjson:\"{tag}\"`").unwrap();
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_field_struct(
    ctx: &mut GoRenderContext,
    name: &str,
    members: &[hir::Member],
    container_annotations: &[hir::Annotation],
) {
    writeln!(&mut ctx.body, "type {name} struct {{").unwrap();
    for member in members {
        let is_optional = member.is_optional();
        let is_skipped = hir::is_skipped(&member.annotations);
        for decl in &member.ident {
            let raw_name = declarator_name(decl);
            let field = go_field_name(raw_name);
            let mut ty = type_with_decl(&member.ty, decl);
            if is_optional {
                ty = pointer_type(&ty);
            }
            let wire_name = if is_skipped {
                "-".to_string()
            } else {
                effective_wire_name(raw_name, &member.annotations, container_annotations)
            };
            writeln!(&mut ctx.body, "\t{field} {ty} `xjson:\"{wire_name}\"`").unwrap();
        }
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
}
