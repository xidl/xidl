use crate::error::IdlcResult;
use crate::generate::rust::util::{
    bitfield_type, render_const, rust_scoped_name, serialize_kind_name,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::BitsetDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_bitset_with_config(self, renderer, &hir::SerializeConfig::default(), &[], &[])
    }
}

pub(crate) fn render_bitset_with_config(
    def: &hir::BitsetDcl,
    renderer: &RustRenderer,
    config: &hir::SerializeConfig,
    module_path: &[String],
    annotations: &[hir::Annotation],
) -> IdlcResult<RustRenderOutput> {
    let parent = def.parent.as_ref().map(rust_scoped_name);
    let fields = def
        .field
        .iter()
        .map(|field| {
            let ty = field
                .ty
                .as_ref()
                .map(bitfield_type)
                .unwrap_or_else(|| "bool".to_string());
            let width = render_const(&field.pos.0);
            json!({
                "ty": ty,
                "name": crate::generate::rust::util::rust_ident(&field.ident),
                "width": width,
            })
        })
        .collect::<Vec<_>>();
    let serialize_kind = serialize_kind_name(def.serialize_kind(config));
    let derive = crate::generate::rust::util::rust_derives_from_annotations_with_extra(
        &def.annotations,
        annotations,
    );
    let module_path = module_path.to_vec();
    let ctx = json!({
        "ident": crate::generate::rust::util::rust_ident(&def.ident),
        "parent": parent,
        "fields": fields,
        "serialize_kind": serialize_kind,
        "derive": derive,
        "module_path": module_path,
        "typeobject_path": renderer.typeobject_path(),
    });
    let rendered = renderer.render_template("bitset.rs.j2", &ctx)?;
    Ok(RustRenderOutput::default().push_source(rendered))
}
