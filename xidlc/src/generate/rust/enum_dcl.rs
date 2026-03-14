use crate::error::IdlcResult;
use crate::generate::rust::util::{
    rust_derive_info_with_extra, rust_passthrough_attrs_from_annotations,
    serde_rename_from_annotations,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::EnumDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let members = self
            .member
            .iter()
            .map(|member| {
                let rust_name = crate::generate::rust::util::rust_ident(&member.ident);
                let rename = serde_rename_from_annotations(&member.annotations);
                let doc = doc_lines_from_annotations(&member.annotations);
                let rust_attrs = rust_passthrough_attrs_from_annotations(&member.annotations);
                json!({
                    "name": rust_name,
                    "serde_rename": rename,
                    "doc": doc,
                    "rust_attrs": rust_attrs,
                })
            })
            .collect::<Vec<_>>();
        let derive = rust_derive_info_with_extra(&self.annotations, &self.annotations);
        let ctx = renderer.enrich_ctx(
            renderer.with_ident(
                json!({
                    "members": members,
                    "has_serde_serialize": derive.has_serde_serialize,
                    "has_serde_deserialize": derive.has_serde_deserialize,
                    "rust_attrs": rust_passthrough_attrs_from_annotations(&self.annotations),
                }),
                &self.ident,
            ),
            &doc_lines_from_annotations(&self.annotations),
        );
        renderer.render_source_template("enum.rs.j2", &ctx)
    }
}
