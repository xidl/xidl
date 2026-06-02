use crate::error::IdlcResult;
use crate::generate::go::{GoRenderContext, ParamDirection};
use convert_case::{Case, Casing};
use std::fmt::Write;
use xidl_parser::hir;

use super::definition_attr::render_attr_types;
use super::definition_names::{go_export_name, go_field_name};
use super::definition_support::{is_out_param, operation_params, param_direction};
use super::definition_type_render::{render_exception, render_type_dcl};
use super::definition_types::{go_const_type, go_type, render_const_expr};

pub(super) fn render_const(
    ctx: &mut GoRenderContext,
    def: &hir::ConstDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    let ty = go_const_type(&def.ty);
    let value = render_const_expr(&def.value)?;
    writeln!(&mut ctx.body, "const {name} {ty} = {value}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

pub(super) fn render_interface(
    ctx: &mut GoRenderContext,
    def: &hir::InterfaceDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let container_annotations = &def.annotations;
    let hir::InterfaceDclInner::InterfaceDef(def) = &def.decl else {
        return Ok(());
    };
    ctx.state.uses_context = true;
    let interface_name = go_export_name(prefix, &def.header.ident);
    let mut methods = Vec::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    let (request_name, response_name) =
                        render_operation_types(ctx, &interface_name, op, container_annotations)?;
                    methods.push((go_field_name(&op.ident), request_name, response_name));
                }
                hir::Export::AttrDcl(attr) => {
                    methods.extend(render_attr_types(ctx, &interface_name, attr)?)
                }
                hir::Export::TypeDcl(type_dcl) => render_type_dcl(ctx, type_dcl, prefix)?,
                hir::Export::ConstDcl(const_dcl) => render_const(ctx, const_dcl, prefix)?,
                hir::Export::ExceptDcl(except) => render_exception(ctx, except, prefix)?,
            }
        }
    }
    writeln!(&mut ctx.body, "type {interface_name} interface {{").unwrap();
    for (method_name, request_name, response_name) in methods {
        writeln!(
            &mut ctx.body,
            "\t{method_name}(ctx context.Context, req *{request_name}) (*{response_name}, error)"
        )
        .unwrap();
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_operation_types(
    ctx: &mut GoRenderContext,
    interface_name: &str,
    op: &hir::OpDcl,
    container_annotations: &[hir::Annotation],
) -> IdlcResult<(String, String)> {
    let request_name = format!(
        "{}{}Request",
        interface_name,
        op.ident.to_case(Case::Pascal)
    );
    let response_name = format!(
        "{}{}Response",
        interface_name,
        op.ident.to_case(Case::Pascal)
    );
    writeln!(&mut ctx.body, "type {request_name} struct {{").unwrap();
    for param in operation_params(op)
        .iter()
        .filter(|param| !is_out_param(param.attr.as_ref()))
    {
        let raw_name = &param.declarator.0;
        let field = go_field_name(raw_name);
        let wire_name = if hir::is_skipped(&param.annotations) {
            "-".to_string()
        } else {
            hir::effective_wire_name(raw_name, &param.annotations, container_annotations)
        };
        writeln!(
            &mut ctx.body,
            "\t{field} {} `xjson:\"{wire_name}\"`",
            go_type(&param.ty),
        )
        .unwrap();
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();

    writeln!(&mut ctx.body, "type {response_name} struct {{").unwrap();
    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        writeln!(&mut ctx.body, "\tReturn {} `xjson:\"return\"`", go_type(ty)).unwrap();
    }
    for param in operation_params(op).iter().filter(|param| {
        matches!(
            param_direction(param.attr.as_ref()),
            ParamDirection::Out | ParamDirection::InOut
        )
    }) {
        let raw_name = &param.declarator.0;
        let field = go_field_name(raw_name);
        let wire_name = if hir::is_skipped(&param.annotations) {
            "-".to_string()
        } else {
            hir::effective_wire_name(raw_name, &param.annotations, container_annotations)
        };
        writeln!(
            &mut ctx.body,
            "\t{field} {} `xjson:\"{wire_name}\"`",
            go_type(&param.ty),
        )
        .unwrap();
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok((request_name, response_name))
}
