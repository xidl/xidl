use crate::error::IdlcResult;
use crate::generate::go::GoRenderContext;
use xidl_parser::hir;

use super::definition_names::go_capitalize;
use super::definition_templates::{
    EmptyStructTemplate, FieldTemplate, MethodTemplate, StructTemplate, render_field_block,
};
use super::definition_types::go_type;

pub(super) fn render_attr_types(
    ctx: &mut GoRenderContext,
    interface_name: &str,
    attr: &hir::AttrDcl,
) -> IdlcResult<Vec<MethodTemplate>> {
    let mut out = Vec::new();
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => {
            if let hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) = &spec.declarator {
                push_get_attr(ctx, &mut out, interface_name, &decl.0, &spec.ty)?;
            }
        }
        hir::AttrDclInner::AttrSpec(spec) => {
            if let hir::AttrDeclarator::SimpleDeclarator(values) = &spec.declarator {
                for decl in values {
                    push_get_attr(ctx, &mut out, interface_name, &decl.0, &spec.ty)?;
                    push_set_attr(ctx, &mut out, interface_name, &decl.0, &spec.ty)?;
                }
            }
        }
    }
    Ok(out)
}

fn push_get_attr(
    ctx: &mut GoRenderContext,
    out: &mut Vec<MethodTemplate>,
    interface_name: &str,
    raw: &str,
    ty: &hir::TypeSpec,
) -> IdlcResult<()> {
    let request_name = format!(
        "{}GetAttribute{}Request",
        interface_name,
        go_capitalize(raw)
    );
    let response_name = format!(
        "{}GetAttribute{}Response",
        interface_name,
        go_capitalize(raw)
    );
    ctx.push_template(
        "empty_struct.go.j2",
        &EmptyStructTemplate {
            name: request_name.clone(),
        },
    )?;
    ctx.push_template(
        "struct.go.j2",
        &StructTemplate {
            name: response_name.clone(),
            fields: render_field_block(
                ctx,
                vec![FieldTemplate {
                    name: "Return".to_string(),
                    ty: go_type(ty),
                    tag: "return".to_string(),
                }],
            )?,
        },
    )?;
    out.push(MethodTemplate {
        name: format!("GetAttribute{}", go_capitalize(raw)),
        request_name,
        response_name,
    });
    Ok(())
}

fn push_set_attr(
    ctx: &mut GoRenderContext,
    out: &mut Vec<MethodTemplate>,
    interface_name: &str,
    raw: &str,
    ty: &hir::TypeSpec,
) -> IdlcResult<()> {
    let request_name = format!(
        "{}SetAttribute{}Request",
        interface_name,
        go_capitalize(raw)
    );
    let response_name = format!(
        "{}SetAttribute{}Response",
        interface_name,
        go_capitalize(raw)
    );
    ctx.push_template(
        "struct.go.j2",
        &StructTemplate {
            name: request_name.clone(),
            fields: render_field_block(
                ctx,
                vec![FieldTemplate {
                    name: "Value".to_string(),
                    ty: go_type(ty),
                    tag: "value".to_string(),
                }],
            )?,
        },
    )?;
    ctx.push_template(
        "empty_struct.go.j2",
        &EmptyStructTemplate {
            name: response_name.clone(),
        },
    )?;
    out.push(MethodTemplate {
        name: format!("SetAttribute{}", go_capitalize(raw)),
        request_name,
        response_name,
    });
    Ok(())
}
