use crate::error::IdlcResult;
use crate::generate::go::{GoRenderContext, ParamDirection};
use xidl_parser::hir;

use super::definition_attr::render_attr_types;
use super::definition_names::{go_capitalize, go_export_name, go_field_name};
use super::definition_support::{is_out_param, operation_params, param_direction};
use super::definition_templates::{
    ConstTemplate, FieldTemplate, InterfaceTemplate, MethodTemplate, OperationTypesTemplate,
    StructTemplate, render_field_block, render_method_block,
};
use super::definition_type_render::{render_exception, render_type_dcl};

/// Naming and annotations shared by an interface's exported declarations.
pub(super) struct InterfaceContext<'a> {
    pub(super) name: String,
    pub(super) annotations: &'a [hir::Annotation],
    pub(super) scope: Vec<String>,
}

pub(super) fn render_const(
    ctx: &mut GoRenderContext,
    def: &hir::ConstDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    let ty = super::go_const_type(&def.ty, ctx.scope(prefix));
    let value = match ctx.integer_constant(def, &name)? {
        Some(value) => value,
        None => super::render_const_expr(&def.value, ctx.scope(prefix)),
    };
    ctx.push_template("const.go.j2", &ConstTemplate { name, ty, value })
}

pub(super) fn render_interface(
    ctx: &mut GoRenderContext,
    def: &hir::InterfaceDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let annotations = &def.annotations;
    let hir::InterfaceDclInner::InterfaceDef(def) = &def.decl else {
        return Ok(());
    };
    let enabled = ctx
        .properties
        .get("enable_interfaces")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    ctx.state.uses_context |= enabled;
    let interface_name = go_export_name(prefix, &def.header.ident);
    let mut scope = prefix.to_vec();
    scope.push(def.header.ident.clone());
    let interface = InterfaceContext {
        name: interface_name.clone(),
        annotations,
        scope,
    };
    let mut methods = Vec::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) if enabled => {
                    methods.push(render_operation_types(ctx, &interface, op)?);
                }
                hir::Export::AttrDcl(attr) if enabled => {
                    methods.extend(render_attr_types(ctx, &interface, attr)?)
                }
                hir::Export::OpDcl(_) | hir::Export::AttrDcl(_) => {}
                hir::Export::TypeDcl(type_dcl) => render_type_dcl(ctx, type_dcl, &interface.scope)?,
                hir::Export::ConstDcl(const_dcl) => render_const(ctx, const_dcl, &interface.scope)?,
                hir::Export::ExceptDcl(except) => render_exception(ctx, except, &interface.scope)?,
            }
        }
    }
    if !enabled {
        return Ok(());
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
    interface: &InterfaceContext<'_>,
    op: &hir::OpDcl,
) -> IdlcResult<MethodTemplate> {
    let interface_name = &interface.name;
    let container_annotations = interface.annotations;
    let request_name = format!("{}{}Request", interface_name, go_capitalize(&op.ident));
    let response_name = format!("{}{}Response", interface_name, go_capitalize(&op.ident));
    let mut request_fields = Vec::new();
    let in_params: Vec<_> = operation_params(op)
        .iter()
        .filter(|param| !is_out_param(param.attr.as_ref()))
        .collect();
    let is_single_request = in_params.len() == 1 && is_composite_type(&in_params[0].ty);

    for param in in_params {
        let raw_name = &param.declarator.0;
        let mut field = go_field_name(raw_name);
        let mut wire_name = if hir::is_skipped(&param.annotations) {
            "-".to_string()
        } else {
            hir::effective_wire_name(raw_name, &param.annotations, container_annotations)
        };

        if is_single_request {
            field = String::new();
            wire_name = ",inline".to_string();
        }

        request_fields.push(FieldTemplate {
            name: field,
            ty: ctx.scope(&interface.scope).type_ref(&param.ty),
            tag: wire_name,
        });
    }

    let mut response_fields = Vec::new();
    let out_params: Vec<_> = operation_params(op)
        .iter()
        .filter(|param| {
            matches!(
                param_direction(param.attr.as_ref()),
                ParamDirection::Out | ParamDirection::InOut
            )
        })
        .collect();

    let is_single_response = if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        out_params.is_empty() && is_composite_type(ty)
    } else {
        out_params.len() == 1 && is_composite_type(&out_params[0].ty)
    };

    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        let mut field = "Return".to_string();
        let mut wire_name = "return".to_string();

        if is_single_response {
            field = String::new();
            wire_name = ",inline".to_string();
        }

        response_fields.push(FieldTemplate {
            name: field,
            ty: ctx.scope(&interface.scope).type_ref(ty),
            tag: wire_name,
        });
    }
    for param in out_params {
        let raw_name = &param.declarator.0;
        let mut field = go_field_name(raw_name);
        let mut wire_name = if hir::is_skipped(&param.annotations) {
            "-".to_string()
        } else {
            hir::effective_wire_name(raw_name, &param.annotations, container_annotations)
        };

        if is_single_response {
            field = String::new();
            wire_name = ",inline".to_string();
        }

        response_fields.push(FieldTemplate {
            name: field,
            ty: ctx.scope(&interface.scope).type_ref(&param.ty),
            tag: wire_name,
        });
    }
    ctx.push_template(
        "operation_types.go.j2",
        &OperationTypesTemplate {
            request: StructTemplate {
                name: request_name.clone(),
                embed: String::new(),
                fields: render_field_block(ctx, request_fields)?,
            },
            response: StructTemplate {
                name: response_name.clone(),
                embed: String::new(),
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

fn is_composite_type(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::IntegerType(_)
        | hir::TypeSpec::FloatingPtType
        | hir::TypeSpec::CharType
        | hir::TypeSpec::WideCharType
        | hir::TypeSpec::Boolean
        | hir::TypeSpec::StringType(_)
        | hir::TypeSpec::WideStringType(_)
        | hir::TypeSpec::FixedPtType(_) => false,
        hir::TypeSpec::ScopedName(_)
        | hir::TypeSpec::SequenceType(_)
        | hir::TypeSpec::MapType(_)
        | hir::TypeSpec::TemplateType(_)
        | hir::TypeSpec::AnyType
        | hir::TypeSpec::ObjectType
        | hir::TypeSpec::ValueBaseType => true,
    }
}
