use crate::error::IdlcResult;
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

use super::GoRestRenderer;

pub(crate) fn render_spec(
    spec: &hir::Specification,
    package_name: &str,
    rest_hir: &RestHirDocument,
    properties: &xidl_parser::hir::ParserProperties,
) -> IdlcResult<String> {
    let renderer = GoRestRenderer::new()?;
    let mut blocks = Vec::new();
    for def in &spec.0 {
        let mut block = String::new();
        render_definition(&mut block, def, &[], &renderer, rest_hir)?;
        if !block.is_empty() {
            blocks.push(block);
        }
    }
    renderer
        .render_template(
            "spec.go.j2",
            &serde_json::json!({
                "package_name": package_name,
                "blocks": blocks,
                "opt": properties,
            }),
        )
        .and_then(|rendered| super::render::format_go_source(&rendered))
}

fn render_definition(
    out: &mut String,
    def: &hir::Definition,
    prefix: &[String],
    renderer: &GoRestRenderer,
    rest_hir: &RestHirDocument,
) -> IdlcResult<()> {
    match def {
        hir::Definition::ModuleDcl(module) => {
            let mut next = prefix.to_vec();
            next.push(module.ident.clone());
            for def in &module.definition {
                render_definition(out, def, &next, renderer, rest_hir)?;
            }
        }
        hir::Definition::InterfaceDcl(interface) => {
            super::interface::render_interface(out, interface, prefix, renderer, rest_hir)?
        }
        _ => {}
    }
    Ok(())
}
