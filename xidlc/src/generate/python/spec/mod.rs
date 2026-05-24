mod spec_render_interface;
mod spec_render_types;
mod spec_types;

use crate::error::IdlcResult;
use crate::generate::python::PythonRenderer;
use serde::Serialize;
use std::fmt::Write;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

use self::spec_render_interface::render_interface;
use self::spec_render_types::{
    render_const, render_constr_type, render_exception, render_type_decl,
};

#[derive(Serialize)]
struct PythonSpecTemplate {
    module_name: String,
    blocks: Vec<String>,
}

pub(crate) fn render_spec(
    spec: &hir::Specification,
    module_name: &str,
    properties: &ParserProperties,
) -> IdlcResult<String> {
    let mut renderer = PythonRenderer::new()?;
    renderer.extend(properties);
    let mut blocks = Vec::new();
    for def in &spec.0 {
        render_definition(&mut blocks, def, &[])?;
    }
    renderer.render_template(
        "spec.py.j2",
        &PythonSpecTemplate {
            module_name: module_name.to_string(),
            blocks,
        },
    )
}

fn render_definition(
    out: &mut Vec<String>,
    def: &hir::Definition,
    prefix: &[String],
) -> IdlcResult<()> {
    match def {
        hir::Definition::ModuleDcl(module) => {
            let mut next = prefix.to_vec();
            next.push(module.ident.clone());
            let mut block = String::new();
            writeln!(block, "# module {}", next.join(".")).unwrap();
            out.push(block);
            for inner in &module.definition {
                render_definition(out, inner, &next)?;
            }
        }
        hir::Definition::ConstDcl(const_dcl) => {
            let mut block = String::new();
            render_const(&mut block, const_dcl);
            out.push(block);
        }
        hir::Definition::ExceptDcl(except_dcl) => {
            let mut block = String::new();
            render_exception(&mut block, except_dcl);
            out.push(block);
        }
        hir::Definition::InterfaceDcl(interface) => {
            let mut block = String::new();
            render_interface(&mut block, interface);
            if !block.is_empty() {
                out.push(block);
            }
        }
        hir::Definition::TypeDcl(type_dcl) => {
            let mut block = String::new();
            render_type_decl(&mut block, type_dcl);
            out.push(block);
        }
        hir::Definition::ConstrTypeDcl(constr) => {
            let mut block = String::new();
            render_constr_type(&mut block, constr);
            out.push(block);
        }
        hir::Definition::Pragma(_) => {}
    }
    Ok(())
}
