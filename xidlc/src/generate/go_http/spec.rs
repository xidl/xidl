use crate::error::IdlcResult;
use xidl_parser::http_hir::HttpHirDocument;
use xidl_parser::hir;

use super::{GoHttpRenderOutput, GoHttpRenderer};

pub(crate) fn render_spec(
    spec: &hir::Specification,
    package_name: &str,
    http_hir: &HttpHirDocument,
) -> IdlcResult<String> {
    let renderer = GoHttpRenderer::new()?;
    let mut out = String::new();
    for def in &spec.0 {
        render_definition(&mut out, def, &[], &renderer, http_hir)?;
    }
    renderer.render_spec(&GoHttpRenderOutput {
        package_name: package_name.to_string(),
        body: out,
    })
}

fn render_definition(
    out: &mut String,
    def: &hir::Definition,
    prefix: &[String],
    renderer: &GoHttpRenderer,
    http_hir: &HttpHirDocument,
) -> IdlcResult<()> {
    match def {
        hir::Definition::ModuleDcl(module) => {
            let mut next = prefix.to_vec();
            next.push(module.ident.clone());
            for def in &module.definition {
                render_definition(out, def, &next, renderer, http_hir)?;
            }
        }
        hir::Definition::InterfaceDcl(interface) => {
            super::interface::render_interface(out, interface, prefix, renderer, http_hir)?
        }
        _ => {}
    }
    Ok(())
}
