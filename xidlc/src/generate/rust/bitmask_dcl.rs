use crate::error::IdlcResult;
use crate::generate::rust::util::{
    rust_derive_info_with_extra, rust_passthrough_attrs_from_annotations,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::BitmaskDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let values = self
            .value
            .iter()
            .enumerate()
            .map(|(idx, value)| {
                json!({
                    "name": crate::generate::rust::util::rust_ident(&value.ident),
                    "index": idx,
                    "doc": doc_lines_from_annotations(&value.annotations),
                    "rust_attrs": rust_passthrough_attrs_from_annotations(&value.annotations),
                })
            })
            .collect::<Vec<_>>();
        let derive = rust_derive_info_with_extra(&self.annotations, &self.annotations);
        let ctx = renderer.enrich_ctx(
            renderer.with_ident(
                json!({
                    "values": values,
                    "derive": derive.all,
                    "rust_attrs": rust_passthrough_attrs_from_annotations(&self.annotations),
                }),
                &self.ident,
            ),
            &doc_lines_from_annotations(&self.annotations),
        );
        renderer.render_source_template("bitmask.rs.j2", &ctx)
    }
}
