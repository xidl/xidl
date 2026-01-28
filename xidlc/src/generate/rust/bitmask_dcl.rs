use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::BitmaskDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_bitmask_with_config(self, renderer, &[], &[])
    }
}

pub(crate) fn render_bitmask_with_config(
    def: &hir::BitmaskDcl,
    renderer: &RustRenderer,
    module_path: &[String],
    annotations: &[hir::Annotation],
) -> IdlcResult<RustRenderOutput> {
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
    let derive = crate::generate::rust::util::rust_derives_from_annotations_with_extra(
        &def.annotations,
        annotations,
    );
    let module_path = module_path.to_vec();
    let ctx = json!({
        "ident": crate::generate::rust::util::rust_ident(&def.ident),
        "values": values,
        "derive": derive,
        "module_path": module_path,
        "typeobject_path": renderer.typeobject_path(),
    });
    let rendered = renderer.render_template("bitmask.rs.j2", &ctx)?;
    Ok(RustRenderOutput::default().push_source(rendered))
}
