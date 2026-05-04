use crate::error::IdlcResult;
use crate::generate::rust::util::{
    bitfield_type, render_const, rust_derive_info_with_extra,
    rust_passthrough_attrs_from_annotations, rust_scoped_name,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::BitsetDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_bitset_with_config(self, renderer)
    }
}

pub(crate) fn render_bitset_with_config(
    def: &hir::BitsetDcl,
    renderer: &RustRenderer,
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
    let derive = rust_derive_info_with_extra(&def.annotations, &def.annotations);
    let ctx = renderer.enrich_ctx(
        json!({
            "ident": crate::generate::rust::util::rust_ident(&def.ident),
            "parent": parent,
            "fields": fields,
            "derive": derive.all,
            "rust_attrs": rust_passthrough_attrs_from_annotations(&def.annotations),
        }),
        &doc_lines_from_annotations(&def.annotations),
    );
    renderer.render_source_template("bitset.rs.j2", &ctx)
}
