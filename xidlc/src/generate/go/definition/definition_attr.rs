use crate::error::IdlcResult;
use crate::generate::go::GoRenderContext;
use convert_case::{Case, Casing};
use std::fmt::Write;
use xidl_parser::hir;

use super::definition_types::go_type;

pub(super) fn render_attr_types(
    ctx: &mut GoRenderContext,
    interface_name: &str,
    attr: &hir::AttrDcl,
) -> IdlcResult<Vec<(String, String, String)>> {
    let mut out = Vec::new();
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => {
            if let hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) = &spec.declarator {
                push_get_attr(ctx, &mut out, interface_name, &decl.0, &spec.ty);
            }
        }
        hir::AttrDclInner::AttrSpec(spec) => {
            if let hir::AttrDeclarator::SimpleDeclarator(values) = &spec.declarator {
                for decl in values {
                    push_get_attr(ctx, &mut out, interface_name, &decl.0, &spec.ty);
                    push_set_attr(ctx, &mut out, interface_name, &decl.0, &spec.ty);
                }
            }
        }
    }
    Ok(out)
}

fn push_get_attr(
    ctx: &mut GoRenderContext,
    out: &mut Vec<(String, String, String)>,
    interface_name: &str,
    raw: &str,
    ty: &hir::TypeSpec,
) {
    let request_name = format!(
        "{}GetAttribute{}Request",
        interface_name,
        raw.to_case(Case::Pascal)
    );
    let response_name = format!(
        "{}GetAttribute{}Response",
        interface_name,
        raw.to_case(Case::Pascal)
    );
    writeln!(&mut ctx.body, "type {request_name} struct {{}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    writeln!(&mut ctx.body, "type {response_name} struct {{").unwrap();
    writeln!(&mut ctx.body, "\tReturn {} `xjson:\"return\"`", go_type(ty)).unwrap();
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    out.push((
        format!("GetAttribute{}", raw.to_case(Case::Pascal)),
        request_name,
        response_name,
    ));
}

fn push_set_attr(
    ctx: &mut GoRenderContext,
    out: &mut Vec<(String, String, String)>,
    interface_name: &str,
    raw: &str,
    ty: &hir::TypeSpec,
) {
    let request_name = format!(
        "{}SetAttribute{}Request",
        interface_name,
        raw.to_case(Case::Pascal)
    );
    let response_name = format!(
        "{}SetAttribute{}Response",
        interface_name,
        raw.to_case(Case::Pascal)
    );
    writeln!(&mut ctx.body, "type {request_name} struct {{").unwrap();
    writeln!(&mut ctx.body, "\tValue {} `xjson:\"value\"`", go_type(ty)).unwrap();
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    writeln!(&mut ctx.body, "type {response_name} struct {{}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    out.push((
        format!("SetAttribute{}", raw.to_case(Case::Pascal)),
        request_name,
        response_name,
    ));
}
