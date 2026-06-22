use crate::error::IdlcResult;
use crate::generate::go::GoRenderContext;
use xidl_parser::hir::{self, effective_wire_name};

use super::definition_names::{go_capitalize, go_export_name, go_field_name, pointer_type};
use super::definition_support::declarator_name;
use super::definition_templates::{
    EnumMemberTemplate, EnumTemplate, ExceptionTemplate, FieldTemplate, StructTemplate,
    TypeDeclTemplate, render_enum_member_block, render_field_block,
};
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
            ctx.push_template(
                "type_decl.go.j2",
                &TypeDeclTemplate {
                    name,
                    ty: "any".to_string(),
                    alias: false,
                },
            )
        }
    }
}

pub(super) fn render_exception(
    ctx: &mut GoRenderContext,
    def: &hir::ExceptDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    ctx.push_template(
        "exception.go.j2",
        &ExceptionTemplate {
            name,
            fields: render_field_block(ctx, field_templates(&def.member, &[]))?,
        },
    )
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
                ctx.push_template(
                    "type_decl.go.j2",
                    &TypeDeclTemplate {
                        name: go_export_name(prefix, &name),
                        ty: base,
                        alias: true,
                    },
                )?;
            }
        }
        hir::TypedefType::ConstrTypeDcl(constr) => {
            render_constr_type(ctx, constr, prefix)?;
            let base = constr_type_name(constr, prefix);
            for decl in &def.decl {
                ctx.push_template(
                    "type_decl.go.j2",
                    &TypeDeclTemplate {
                        name: go_export_name(prefix, declarator_name(decl)),
                        ty: base.clone(),
                        alias: true,
                    },
                )?;
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
        hir::ConstrTypeDcl::BitmaskDcl(def) => ctx.push_template(
            "type_decl.go.j2",
            &TypeDeclTemplate {
                name: go_export_name(prefix, &def.ident),
                ty: "uint64".to_string(),
                alias: false,
            },
        ),
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
        go_export_name(prefix, &def.ident),
        &def.member,
        &def.annotations,
    )
}

fn render_enum(ctx: &mut GoRenderContext, def: &hir::EnumDcl, prefix: &[String]) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    let mut members = Vec::new();
    for member in &def.member {
        let value_name = format!("{}{}", name, go_capitalize(&member.ident));
        let wire_name = effective_wire_name(&member.ident, &member.annotations, &def.annotations);
        members.push(EnumMemberTemplate {
            name: value_name,
            enum_name: name.clone(),
            wire_name,
        });
    }
    ctx.push_template(
        "enum.go.j2",
        &EnumTemplate {
            name,
            members: render_enum_member_block(ctx, members)?,
        },
    )
}

fn render_simple_struct(
    ctx: &mut GoRenderContext,
    name: &str,
    fields: &[(&str, &str, &str)],
) -> IdlcResult<()> {
    ctx.push_template(
        "struct.go.j2",
        &StructTemplate {
            name: name.to_string(),
            fields: render_field_block(
                ctx,
                fields
                    .iter()
                    .map(|(name, ty, tag)| FieldTemplate {
                        name: (*name).to_string(),
                        ty: (*ty).to_string(),
                        tag: (*tag).to_string(),
                    })
                    .collect(),
            )?,
        },
    )
}

fn render_field_struct(
    ctx: &mut GoRenderContext,
    name: String,
    members: &[hir::Member],
    container_annotations: &[hir::Annotation],
) -> IdlcResult<()> {
    ctx.push_template(
        "struct.go.j2",
        &StructTemplate {
            name,
            fields: render_field_block(ctx, field_templates(members, container_annotations))?,
        },
    )
}

fn field_templates(
    members: &[hir::Member],
    container_annotations: &[hir::Annotation],
) -> Vec<FieldTemplate> {
    let mut fields = Vec::new();
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
            fields.push(FieldTemplate {
                name: field,
                ty,
                tag: wire_name,
            });
        }
    }
    fields
}
