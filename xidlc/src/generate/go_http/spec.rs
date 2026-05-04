use crate::error::IdlcResult;
use xidl_parser::hir;
use xidl_parser::http_hir::HttpHirDocument;

use super::{GoHttpRenderOutput, GoHttpRenderer};

pub(crate) fn render_spec(
    spec: &hir::Specification,
    package_name: &str,
    http_hir: &HttpHirDocument,
) -> IdlcResult<String> {
    let renderer = GoHttpRenderer::new()?;
    let mut blocks = Vec::new();
    for def in &spec.0 {
        let mut block = String::new();
        render_definition(&mut block, def, &[], &renderer, http_hir)?;
        if !block.is_empty() {
            blocks.push(block);
        }
    }
    renderer.render_spec(&GoHttpRenderOutput {
        package_name: package_name.to_string(),
        blocks,
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
