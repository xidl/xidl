use crate::error::IdlcResult;
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

use super::{
    GoRestRenderBlocks, GoRestRenderContext, GoRestRenderMode, GoRestRenderOutput, GoRestRenderer,
};

pub(crate) fn render_spec(
    spec: &hir::Specification,
    package_name: &str,
    rest_hir: &RestHirDocument,
    properties: &xidl_parser::hir::ParserProperties,
) -> IdlcResult<String> {
    let renderer = GoRestRenderer::new()?;
    let context = GoRestRenderContext {
        renderer: &renderer,
        rest_hir,
        mode: GoRestRenderMode::from_properties(properties)?,
    };
    let mut template_properties = properties.clone();
    template_properties.insert(
        "enable_client".to_string(),
        context.mode.enable_client.into(),
    );
    template_properties.insert(
        "enable_server".to_string(),
        context.mode.enable_server.into(),
    );
    let mut blocks = GoRestRenderBlocks::default();
    for def in &spec.0 {
        blocks.extend(render_definition(def, &[], &context)?);
    }
    renderer.render_spec(&GoRestRenderOutput {
        package_name: package_name.to_string(),
        blocks,
        opt: template_properties,
    })
}

fn render_definition(
    def: &hir::Definition,
    prefix: &[String],
    context: &GoRestRenderContext<'_>,
) -> IdlcResult<GoRestRenderBlocks> {
    let mut blocks = GoRestRenderBlocks::default();
    match def {
        hir::Definition::ModuleDcl(module) => {
            let mut next = prefix.to_vec();
            next.push(module.ident.clone());
            for def in &module.definition {
                blocks.extend(render_definition(def, &next, context)?);
            }
        }
        hir::Definition::InterfaceDcl(interface) => {
            blocks.extend(super::interface::render_interface(
                interface, prefix, context,
            )?);
        }
        _ => {}
    }
    Ok(blocks)
}
