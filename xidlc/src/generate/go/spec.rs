use crate::error::IdlcResult;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

use super::{GoRenderContext, GoRenderer};

pub(crate) fn render_spec(
    spec: &hir::Specification,
    package_name: &str,
    properties: &ParserProperties,
) -> IdlcResult<String> {
    let mut ctx = GoRenderContext::new(package_name.to_string(), properties.clone());
    definition::render_spec(&mut ctx, spec)?;
    let output = ctx.finish();
    let renderer = GoRenderer::new()?;
    renderer.render_spec(&output)
}

mod definition {
    pub(crate) use super::super::definition::render_spec;
}
