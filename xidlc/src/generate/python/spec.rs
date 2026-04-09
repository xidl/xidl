#[path = "spec_render_interface.rs"]
mod spec_render_interface;
#[path = "spec_render_types.rs"]
mod spec_render_types;
#[path = "spec_types.rs"]
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
    body: String,
}

pub(crate) fn render_spec(
    spec: &hir::Specification,
    module_name: &str,
    _properties: &ParserProperties,
) -> IdlcResult<String> {
    let renderer = PythonRenderer::new()?;
    let mut body = String::new();
    for def in &spec.0 {
        render_definition(&mut body, def, &[])?;
    }
    renderer.render_template(
        "spec.py.j2",
        &PythonSpecTemplate {
            module_name: module_name.to_string(),
            body,
        },
    )
}

fn render_definition(out: &mut String, def: &hir::Definition, prefix: &[String]) -> IdlcResult<()> {
    match def {
        hir::Definition::ModuleDcl(module) => {
            let mut next = prefix.to_vec();
            next.push(module.ident.clone());
            writeln!(out, "# module {}", next.join(".")).unwrap();
            for inner in &module.definition {
                render_definition(out, inner, &next)?;
            }
        }
        hir::Definition::ConstDcl(const_dcl) => render_const(out, const_dcl),
        hir::Definition::ExceptDcl(except_dcl) => render_exception(out, except_dcl),
        hir::Definition::InterfaceDcl(interface) => render_interface(out, interface),
        hir::Definition::TypeDcl(type_dcl) => render_type_decl(out, type_dcl),
        hir::Definition::ConstrTypeDcl(constr) => render_constr_type(out, constr),
        hir::Definition::Pragma(_) => {}
    }
    Ok(())
}
