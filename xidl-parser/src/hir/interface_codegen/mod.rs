mod attrs;
mod context;
mod hash;
mod ops;
mod render;
mod types;

use super::*;
use crate::error::ParserResult;
use context::{ConstContext, TemplateContext};
use hash::{exception_hash_const_name, hash_string, op_hash_const_name};
use ops::{collect_operations, OperationInfo};
use std::collections::HashSet;

pub trait ToTemplateContext {
    fn to_template_context(&self, modules: &[String]) -> ParserResult<Option<TemplateContext>>;
}

impl ToTemplateContext for InterfaceDcl {
    fn to_template_context(&self, modules: &[String]) -> ParserResult<Option<TemplateContext>> {
        let InterfaceDcl::InterfaceDef(def) = self else {
            return Ok(None);
        };
        let body = match def.interface_body.as_ref() {
            Some(body) => body,
            None => return Ok(None),
        };

        let ops = collect_operations(body);
        if ops.is_empty() {
            return Ok(None);
        }

        let consts = collect_consts(&def.header.ident, modules, &ops);
        let operations = ops
            .iter()
            .map(|op| op.to_context())
            .collect::<Vec<_>>();

        Ok(Some(TemplateContext {
            modules: modules.to_vec(),
            interface_name: def.header.ident.clone(),
            consts,
            operations,
        }))
    }
}

pub fn expand_interface(interface: &InterfaceDcl, modules: &[String]) -> ParserResult<Vec<Definition>> {
    let Some(ctx) = interface.to_template_context(modules)? else {
        return Ok(Vec::new());
    };

    let idl = render::render_template("interface.idl.j2", &ctx)?;
    let typed = crate::parser::parser_text(&idl)?;
    let spec = super::spec_from_typed_ast(typed, false);
    Ok(spec.0)
}

fn collect_consts(
    interface_name: &str,
    modules: &[String],
    ops: &[OperationInfo],
) -> Vec<ConstContext> {
    let mut consts = Vec::new();
    let mut seen_exception_consts = HashSet::new();

    for op in ops {
        let op_hash_name = op_hash_const_name(interface_name, &op.name);
        consts.push(ConstContext {
            name: op_hash_name,
            value: hash_string(&op.name).to_string(),
        });

        for exception in &op.raises {
            let const_name = exception_hash_const_name(exception);
            if !seen_exception_consts.insert(const_name.clone()) {
                continue;
            }
            let full_name = types::qualified_exception_name(exception, modules);
            consts.push(ConstContext {
                name: const_name,
                value: hash_string(&full_name).to_string(),
            });
        }
    }

    consts
}
