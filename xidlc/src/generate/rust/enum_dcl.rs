use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::EnumDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_enum_with_config(self, renderer, &[])
    }
}

pub(crate) fn render_enum_with_config(
    def: &hir::EnumDcl,
    renderer: &RustRenderer,
    module_path: &[String],
) -> IdlcResult<RustRenderOutput> {
    let members = def
        .member
        .iter()
        .map(|member| crate::generate::rust::util::rust_ident(&member.ident))
        .collect::<Vec<_>>();
    let module_path = module_path.to_vec();
    let ctx = json!({
        "ident": crate::generate::rust::util::rust_ident(&def.ident),
        "members": members,
        "module_path": module_path,
        "typeobject_path": renderer.typeobject_path(),
    });
    let rendered = renderer.render_template("enum.rs.j2", &ctx)?;
    Ok(RustRenderOutput::default().push_source(rendered))
}
