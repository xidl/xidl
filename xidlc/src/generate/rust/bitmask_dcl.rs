use crate::error::IdlcResult;
use crate::generate::rust::typeobject_runtime;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::BitmaskDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_bitmask_with_config(self, renderer, &[])
    }
}

pub(crate) fn render_bitmask_with_config(
    def: &hir::BitmaskDcl,
    renderer: &RustRenderer,
    module_path: &[String],
) -> IdlcResult<RustRenderOutput> {
    let type_object = typeobject_runtime::typeobject_for_bitmask(def, module_path);
    let values = def
        .value
        .iter()
        .enumerate()
        .map(|(idx, value)| {
            json!({
                "name": crate::generate::rust::util::rust_ident(&value.ident),
                "index": idx,
            })
        })
        .collect::<Vec<_>>();
    let module_path = module_path.to_vec();
    let ctx = json!({
        "ident": crate::generate::rust::util::rust_ident(&def.ident),
        "values": values,
        "module_path": module_path,
        "typeobject_path": renderer.typeobject_path(),
        "typeobject_complete": type_object.complete,
        "typeobject_minimal": type_object.minimal,
    });
    let rendered = renderer.render_template("bitmask.rs.j2", &ctx)?;
    Ok(RustRenderOutput::default().push_source(rendered))
}
