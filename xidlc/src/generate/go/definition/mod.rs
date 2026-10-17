mod definition_attr;
mod definition_names;
mod definition_render;
mod definition_support;
mod definition_templates;
mod definition_type_render;
mod definition_types;

use crate::error::IdlcResult;
use std::collections::BTreeSet;
use xidl_parser::hir;

#[allow(unused_imports)]
pub(crate) use self::definition_names::{go_export_name, go_field_name, pointer_type};
#[allow(unused_imports)]
pub(crate) use self::definition_support::{
    declarator_name, is_out_param, operation_params, param_direction,
};
#[allow(unused_imports)]
pub(crate) use self::definition_types::{
    constr_type_name, go_const_type, go_literal, go_scoped_name, go_type_with_prefix,
    render_const_expr,
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

pub(crate) fn collect_type_paths(spec: &hir::Specification) -> BTreeSet<Vec<String>> {
    let mut paths = BTreeSet::new();
    collect_definition_type_paths(&spec.0, &[], &mut paths);
    paths
}

fn collect_definition_type_paths(
    definitions: &[hir::Definition],
    prefix: &[String],
    paths: &mut BTreeSet<Vec<String>>,
) {
    for definition in definitions {
        match definition {
            hir::Definition::ModuleDcl(module) => {
                let mut next = prefix.to_vec();
                next.push(module.ident.clone());
                collect_definition_type_paths(&module.definition, &next, paths);
            }
            hir::Definition::TypeDcl(value) => collect_type_decl_path(value, prefix, paths),
            hir::Definition::ExceptDcl(value) => {
                let mut path = prefix.to_vec();
                path.push(value.ident.clone());
                paths.insert(path);
            }
            hir::Definition::InterfaceDcl(value) => {
                let hir::InterfaceDclInner::InterfaceDef(interface) = &value.decl else {
                    continue;
                };
                if let Some(body) = &interface.interface_body {
                    let mut interface_prefix = prefix.to_vec();
                    interface_prefix.push(interface.header.ident.clone());
                    for export in &body.0 {
                        match export {
                            hir::Export::TypeDcl(value) => {
                                collect_type_decl_path(value, &interface_prefix, paths)
                            }
                            hir::Export::ExceptDcl(value) => {
                                let mut path = interface_prefix.clone();
                                path.push(value.ident.clone());
                                paths.insert(path);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_type_decl_path(
    declaration: &hir::TypeDcl,
    prefix: &[String],
    paths: &mut BTreeSet<Vec<String>>,
) {
    match declaration {
        hir::TypeDcl::ConstrTypeDcl(value) => {
            let name = match value {
                hir::ConstrTypeDcl::StructDcl(value) => &value.ident,
                hir::ConstrTypeDcl::StructForwardDcl(value) => &value.ident,
                hir::ConstrTypeDcl::EnumDcl(value) => &value.ident,
                hir::ConstrTypeDcl::UnionDef(value) => &value.ident,
                hir::ConstrTypeDcl::UnionForwardDcl(value) => &value.ident,
                hir::ConstrTypeDcl::BitsetDcl(value) => &value.ident,
                hir::ConstrTypeDcl::BitmaskDcl(value) => &value.ident,
            };
            let mut path = prefix.to_vec();
            path.push(name.clone());
            paths.insert(path);
        }
        hir::TypeDcl::TypedefDcl(value) => {
            for declaration in &value.decl {
                let mut path = prefix.to_vec();
                path.push(declarator_name(declaration).to_string());
                paths.insert(path);
            }
        }
        hir::TypeDcl::NativeDcl(value) => {
            let mut path = prefix.to_vec();
            path.push(value.decl.0.clone());
            paths.insert(path);
        }
    }
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
