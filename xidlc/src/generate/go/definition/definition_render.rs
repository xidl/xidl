use crate::error::IdlcResult;
use crate::generate::go::{GoRenderContext, ParamDirection};
use convert_case::{Case, Casing};
use xidl_parser::hir;

use super::definition_attr::render_attr_types;
use super::definition_names::{go_export_name, go_field_name};
use super::definition_support::{is_out_param, operation_params, param_direction};
use super::definition_templates::{
    ConstTemplate, FieldTemplate, InterfaceTemplate, MethodTemplate, OperationTypesTemplate,
    StructTemplate, render_field_block, render_method_block,
};
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
    ctx.push_template("const.go.j2", &ConstTemplate { name, ty, value })
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
                    methods.push(render_operation_types(
                        ctx,
                        &interface_name,
                        op,
                        container_annotations,
                    )?);
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
    ctx.push_template(
        "interface.go.j2",
        &InterfaceTemplate {
            name: interface_name,
            methods: render_method_block(ctx, &methods)?,
        },
    )
}

fn render_operation_types(
    ctx: &mut GoRenderContext,
    interface_name: &str,
    op: &hir::OpDcl,
    container_annotations: &[hir::Annotation],
) -> IdlcResult<MethodTemplate> {
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
    let mut request_fields = Vec::new();
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
        request_fields.push(FieldTemplate {
            name: field,
            ty: go_type(&param.ty),
            tag: wire_name,
        });
    }

    let mut response_fields = Vec::new();
    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        response_fields.push(FieldTemplate {
            name: "Return".to_string(),
            ty: go_type(ty),
            tag: "return".to_string(),
        });
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
        response_fields.push(FieldTemplate {
            name: field,
            ty: go_type(&param.ty),
            tag: wire_name,
        });
    }
    ctx.push_template(
        "operation_types.go.j2",
        &OperationTypesTemplate {
            request: StructTemplate {
                name: request_name.clone(),
                fields: render_field_block(ctx, request_fields)?,
            },
            response: StructTemplate {
                name: response_name.clone(),
                fields: render_field_block(ctx, response_fields)?,
            },
        },
    )?;
    Ok(MethodTemplate {
        name: go_field_name(&op.ident),
        request_name,
        response_name,
    })
}
