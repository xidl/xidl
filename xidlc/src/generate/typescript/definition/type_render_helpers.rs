use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::utils::doc_lines_from_annotations;
use xidl_parser::hir;

use super::contexts::{TypedefTypeContext, TypedefZodContext};
use super::names::ts_ident;
use super::output::TsRenderOutput;

pub(crate) fn render_native(
    out: &mut TsRenderOutput,
    native: &hir::NativeDcl,
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let name = ts_ident(&native.decl.0);
    out.types.push(renderer.render_template(
        "typedef.d.ts.j2",
        &TypedefTypeContext {
            name: name.clone(),
            type_expr: "unknown".to_string(),
            doc: doc_lines_from_annotations(&native.annotations),
        },
    )?);
    out.zod.push(renderer.render_template(
        "typedef.zod.ts.j2",
        &TypedefZodContext {
            schema_name: format!("{name}Schema"),
            name,
            schema_expr: "z.unknown()".to_string(),
        },
    )?);
    Ok(())
}

pub(crate) fn render_bit_number(
    ident: &str,
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let name = ts_ident(ident);
    Ok(TsRenderOutput {
        types: vec![renderer.render_template(
            "typedef.d.ts.j2",
            &TypedefTypeContext {
                name: name.clone(),
                type_expr: "number".to_string(),
                doc: Vec::new(),
            },
        )?],
        zod: vec![renderer.render_template(
            "typedef.zod.ts.j2",
            &TypedefZodContext {
                schema_name: format!("{name}Schema"),
                name,
                schema_expr: "z.number().int()".to_string(),
            },
        )?],
        client: Vec::new(),
    })
}
