mod definition_attr;
mod definition_names;
mod definition_render;
mod definition_support;
mod definition_templates;
mod definition_type_render;
mod definition_types;

use crate::error::IdlcResult;
use xidl_parser::hir;

#[allow(unused_imports)]
pub(crate) use self::definition_names::{go_export_name, go_field_name, pointer_type};
#[allow(unused_imports)]
pub(crate) use self::definition_support::{
    declarator_name, is_out_param, operation_params, param_direction,
};
#[allow(unused_imports)]
pub(crate) use self::definition_types::{
    constr_type_name, go_const_type, go_literal, go_scoped_name, go_type, render_const_expr,
    type_with_decl,
};
use super::GoRenderContext;

pub(crate) fn render_spec(
    ctx: &mut GoRenderContext,
    spec: &hir::Specification,
) -> IdlcResult<Vec<String>> {
    let mut blocks = Vec::new();
    for def in &spec.0 {
        let mut body = String::new();
        std::mem::swap(&mut body, &mut ctx.body);
        render_definition(ctx, def, &[])?;
        let block = std::mem::take(&mut ctx.body);
        ctx.body = body;
        if !block.is_empty() {
            blocks.push(block);
        }
    }
    Ok(blocks)
}

fn render_definition(
    ctx: &mut GoRenderContext,
    def: &hir::Definition,
    prefix: &[String],
) -> IdlcResult<()> {
    match def {
        hir::Definition::ModuleDcl(module) => {
            let mut next = prefix.to_vec();
            next.push(module.ident.clone());
            for def in &module.definition {
                render_definition(ctx, def, &next)?;
            }
        }
        hir::Definition::TypeDcl(ty) => definition_type_render::render_type_dcl(ctx, ty, prefix)?,
        hir::Definition::ConstDcl(const_dcl) => {
            definition_render::render_const(ctx, const_dcl, prefix)?
        }
        hir::Definition::ExceptDcl(except) => {
            definition_type_render::render_exception(ctx, except, prefix)?
        }
        hir::Definition::InterfaceDcl(interface) => {
            if ctx
                .properties
                .get("enable_interfaces")
                .and_then(|value| value.as_bool())
                .unwrap_or(true)
            {
                definition_render::render_interface(ctx, interface, prefix)?;
            }
        }
        _ => {}
    }
    Ok(())
}
