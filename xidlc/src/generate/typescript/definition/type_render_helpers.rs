use crate::error::IdlcResult;
use crate::generate::typescript::definition::contexts::TsRenderCtx;
use crate::generate::utils::doc_lines_from_annotations;
use xidl_parser::hir;

use super::contexts::{TsType, TypedefTypeContext, TypedefZodContext, ZodSchema};
use super::names::ts_ident;
use super::output::TsRenderOutput;

pub(crate) fn render_native(
    out: &mut TsRenderOutput,
    native: &hir::NativeDcl,
    ctx: &TsRenderCtx<'_>,
) -> IdlcResult<()> {
    let name = ts_ident(&native.decl.0);
    out.types.push(ctx.renderer.render_template(
        "typedef.d.ts.j2",
        &TypedefTypeContext {
            name: name.clone(),
            type_expr: TsType::Any,
            doc: doc_lines_from_annotations(&native.annotations),
        },
    )?);
    out.zod.push(ctx.renderer.render_template(
        "typedef.zod.ts.j2",
        &TypedefZodContext {
            schema_name: name.clone(),
            name,
            schema_expr: ZodSchema::Any,
            recursive: false,
            type_path: String::new(),
            file_stem: ctx.file_stem.to_string(),
        },
    )?);
    Ok(())
}

pub(crate) fn render_bit_number(ident: &str, ctx: &TsRenderCtx<'_>) -> IdlcResult<TsRenderOutput> {
    let name = ts_ident(ident);
    Ok(TsRenderOutput {
        types: vec![ctx.renderer.render_template(
            "typedef.d.ts.j2",
            &TypedefTypeContext {
                name: name.clone(),
                type_expr: TsType::Primitive("number".into()),
                doc: Vec::new(),
            },
        )?],
        zod: vec![ctx.renderer.render_template(
            "typedef.zod.ts.j2",
            &TypedefZodContext {
                schema_name: name.clone(),
                name,
                schema_expr: ZodSchema::Primitive("number().int()".into()),
                recursive: false,
                type_path: String::new(),
                file_stem: ctx.file_stem.to_string(),
            },
        )?],
        client: Vec::new(),
    })
}
