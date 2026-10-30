use super::annotations::{ts_deprecation, ts_doc_lines};
use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::contexts::TsRenderCtx;
use xidl_parser::hir;

use super::contexts::{
    EnumTypeContext, EnumZodContext, FieldTypeContext, FieldZodContext, StructTypeContext,
    StructZodContext, TypedefTypeContext, TypedefZodContext, UnionTypeContext, UnionZodContext,
};
use super::method::TypeRefTarget;
use super::names::{declarator_name, ts_ident};
use super::output::TsRenderOutput;
use super::struct_fields::struct_fields;
use super::type_expr::{
    apply_array_ts, apply_array_zod, ts_type_for_constr_inline, ts_type_for_decl,
    zod_schema_for_constr_inline, zod_schema_for_decl,
};
use super::type_render_helpers::{render_bit_number, render_native};

pub(crate) fn render_constr_type(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
    ctx: &TsRenderCtx<'_>,
) -> IdlcResult<TsRenderOutput> {
    let mut out = TsRenderOutput::default();
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => out.extend(render_struct(def, module_path, ctx)?),
        hir::ConstrTypeDcl::EnumDcl(def) => out.extend(render_enum(def, ctx.renderer)?),
        hir::ConstrTypeDcl::UnionDef(def) => out.extend(render_union(def, module_path, ctx)?),
        hir::ConstrTypeDcl::BitsetDcl(def) => out.extend(render_bit_number(&def.ident, ctx)?),
        hir::ConstrTypeDcl::BitmaskDcl(def) => out.extend(render_bit_number(&def.ident, ctx)?),
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {}
    }
    Ok(out)
}

pub(crate) fn render_exception(
    except: &hir::ExceptDcl,
    module_path: &[String],
    ctx: &TsRenderCtx<'_>,
) -> IdlcResult<TsRenderOutput> {
    render_struct(
        &hir::StructDcl {
            annotations: except.annotations.clone(),
            ident: except.ident.clone(),
            parent: Vec::new(),
            member: except.member.clone(),
            inherited: Vec::new(),
        },
        module_path,
        ctx,
    )
}

pub(crate) fn render_type_dcl(
    ty: &hir::TypeDcl,
    module_path: &[String],
    ctx: &TsRenderCtx<'_>,
) -> IdlcResult<TsRenderOutput> {
    let mut out = TsRenderOutput::default();
    match ty {
        hir::TypeDcl::ConstrTypeDcl(constr) => {
            out.extend(render_constr_type(constr, module_path, ctx)?)
        }
        hir::TypeDcl::TypedefDcl(typedef) => render_typedefs(&mut out, typedef, module_path, ctx)?,
        hir::TypeDcl::NativeDcl(native) => render_native(&mut out, native, ctx)?,
    }
    Ok(out)
}

fn render_typedefs(
    out: &mut TsRenderOutput,
    typedef: &hir::TypedefDcl,
    module_path: &[String],
    ctx: &TsRenderCtx<'_>,
) -> IdlcResult<()> {
    for decl in &typedef.decl {
        let name = ts_ident(declarator_name(decl));
        let type_expr = match &typedef.ty {
            hir::TypedefType::TypeSpec(spec) => {
                ts_type_for_decl(spec, decl, module_path, TypeRefTarget::Types)
            }
            hir::TypedefType::ConstrTypeDcl(constr) => {
                let base = ts_type_for_constr_inline(constr, module_path, TypeRefTarget::Types);
                apply_array_ts(base, decl)
            }
        };
        out.types.push(ctx.renderer.render_template(
            "typedef.d.ts.j2",
            &TypedefTypeContext {
                name: name.clone(),
                type_expr,
                doc: ts_doc_lines(&typedef.annotations)?,
            },
        )?);
        let schema_expr = match &typedef.ty {
            hir::TypedefType::TypeSpec(spec) => zod_schema_for_decl(spec, decl, module_path),
            hir::TypedefType::ConstrTypeDcl(constr) => {
                let base = zod_schema_for_constr_inline(constr, module_path);
                apply_array_zod(base, decl)
            }
        };
        out.zod.push(ctx.renderer.render_template(
            "typedef.zod.ts.j2",
            &TypedefZodContext {
                schema_name: name.clone(), // Template will add 'Schema' suffix
                name,
                schema_expr,
                recursive: is_recursive(ctx, module_path, declarator_name(decl)),
                type_path: type_path(module_path, declarator_name(decl)),
                file_stem: ctx.file_stem.to_string(),
            },
        )?);
    }
    Ok(())
}

fn render_struct(
    def: &hir::StructDcl,
    module_path: &[String],
    ctx: &TsRenderCtx<'_>,
) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    let extends = if def.parent.is_empty() {
        None
    } else {
        Some(
            def.parent
                .iter()
                .map(|parent| {
                    super::type_expr::ts_type_for_type_spec(
                        &hir::TypeSpec::ScopedName(parent.clone()),
                        module_path,
                        TypeRefTarget::Types,
                    )
                })
                .collect(),
        )
    };
    let fields = struct_fields(&def.member, &def.annotations, module_path)?;
    let wire_fields = struct_fields(
        &def.members().cloned().collect::<Vec<_>>(),
        &def.annotations,
        module_path,
    )?;
    let types = ctx.renderer.render_template(
        "struct.d.ts.j2",
        &StructTypeContext {
            ident: ident.clone(),
            extends,
            doc: ts_doc_lines(&def.annotations)?,
            fields: fields
                .iter()
                .map(|field| FieldTypeContext {
                    prop: field.prop.clone(),
                    ty: field.ty.clone(),
                    optional: field.optional,
                    doc: field.doc.clone(),
                })
                .collect(),
        },
    )?;
    let zod = ctx.renderer.render_template(
        "struct.zod.ts.j2",
        &StructZodContext {
            schema_name: ident.clone(), // Template will add 'Schema' suffix
            ident,
            fields: wire_fields
                .into_iter()
                .map(|field| FieldZodContext {
                    prop: field.prop,
                    schema: field.schema,
                    optional: field.optional,
                    xjson_meta: field.xjson_meta,
                })
                .collect(),
            recursive: is_recursive(ctx, module_path, &def.ident),
            type_path: type_path(module_path, &def.ident),
            file_stem: ctx.file_stem.to_string(),
        },
    )?;
    Ok(TsRenderOutput {
        types: vec![types],
        zod: vec![zod],
        client: Vec::new(),
    })
}

fn render_enum(def: &hir::EnumDcl, renderer: &TypescriptRenderer) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    // String-literal unions have no per-value JSDoc slot, so deprecated
    // enumerators are recorded on the type.
    let mut enum_doc = ts_doc_lines(&def.annotations)?;
    let mut members = Vec::new();
    for value in &def.member {
        let wire = hir::effective_wire_name(&value.ident, &value.annotations, &def.annotations);
        if let Some(info) = ts_deprecation(&value.annotations)? {
            enum_doc.push(info.enumerator_note(&wire));
        }
        members.push(wire);
    }
    Ok(TsRenderOutput {
        types: vec![renderer.render_template(
            "enum.d.ts.j2",
            &EnumTypeContext {
                ident: ident.clone(),
                members,
                doc: enum_doc,
            },
        )?],
        zod: vec![
            renderer.render_template(
                "enum.zod.ts.j2",
                &EnumZodContext {
                    schema_name: ident.clone(), // Template will add 'Schema' suffix
                    ident,
                    values: def
                        .member
                        .iter()
                        .map(|value| {
                            hir::effective_wire_name(
                                &value.ident,
                                &value.annotations,
                                &def.annotations,
                            )
                        })
                        .collect(),
                },
            )?,
        ],
        client: Vec::new(),
    })
}

fn render_union(
    def: &hir::UnionDef,
    module_path: &[String],
    ctx: &TsRenderCtx<'_>,
) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    let projection = super::union::TsUnion {
        definition: def,
        module_path,
    };
    let variants = projection.types(TypeRefTarget::Types);
    let schema_variants = projection.schemas();
    Ok(TsRenderOutput {
        types: vec![ctx.renderer.render_template(
            "union.d.ts.j2",
            &UnionTypeContext {
                ident: ident.clone(),
                variants,
                doc: ts_doc_lines(&def.annotations)?,
            },
        )?],
        zod: vec![ctx.renderer.render_template(
            "union.zod.ts.j2",
            &UnionZodContext {
                schema_name: ident.clone(), // Template will add 'Schema' suffix
                ident,
                variants: schema_variants,
                recursive: is_recursive(ctx, module_path, &def.ident),
                type_path: type_path(module_path, &def.ident),
                file_stem: ctx.file_stem.to_string(),
            },
        )?],
        client: Vec::new(),
    })
}

fn is_recursive(ctx: &TsRenderCtx<'_>, module_path: &[String], ident: &str) -> bool {
    let mut path = module_path.to_vec();
    path.push(ident.to_string());
    ctx.recursive.contains(&path)
}

fn type_path(module_path: &[String], ident: &str) -> String {
    let mut parts = module_path
        .iter()
        .map(|part| ts_ident(part))
        .collect::<Vec<_>>();
    parts.push(ts_ident(ident));
    parts.join(".")
}
